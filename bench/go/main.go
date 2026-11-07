// Command scan is the Go half of the spec-dna language benchmark.
//
// It walks a tree, parses Markdown front matter, links and id tokens, and
// resolves them. The Rust half in ../rust implements the same rules; see
// ../README.md. Standard library only.
package main

import (
	"bufio"
	"fmt"
	"io/fs"
	"os"
	"path"
	"path/filepath"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"sync"
	"unsafe"
)

const version = "0.1.0"
const indexMagic = "SDNAIDX1"

type doc struct {
	path   string
	size   int64
	mtime  int64
	id     string
	links  []string
	idrefs []string
	cached bool
}

func main() {
	args := os.Args[1:]
	if len(args) == 1 && args[0] == "--version" {
		fmt.Println("scan-go", version)
		return
	}
	var root, index string
	list := false
	excl := map[string]bool{".git": true, "node_modules": true}
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--index":
			i++
			index = args[i]
		case "--exclude":
			i++
			excl[args[i]] = true
		case "--list":
			list = true
		default:
			root = args[i]
		}
	}
	if root == "" {
		fmt.Fprintln(os.Stderr, "usage: scan [--index FILE] [--exclude NAME] [--list] ROOT")
		os.Exit(2)
	}
	if err := run(root, index, excl, list); err != nil {
		fmt.Fprintln(os.Stderr, "scan:", err)
		os.Exit(1)
	}
}

func run(root, index string, excl map[string]bool, list bool) error {
	files := map[string]bool{}
	dirs := map[string]bool{"": true}
	var docs []*doc
	err := filepath.WalkDir(root, func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, _ := filepath.Rel(root, p)
		rel = filepath.ToSlash(rel)
		if d.IsDir() {
			if p != root && excl[d.Name()] {
				return filepath.SkipDir
			}
			if rel != "." {
				dirs[rel] = true
			}
			return nil
		}
		if !d.Type().IsRegular() {
			return nil
		}
		files[rel] = true
		if strings.HasSuffix(rel, ".md") {
			info, err := d.Info()
			if err != nil {
				return err
			}
			docs = append(docs, &doc{path: rel, size: info.Size(), mtime: info.ModTime().UnixNano()})
		}
		return nil
	})
	if err != nil {
		return err
	}
	sort.Slice(docs, func(i, j int) bool { return docs[i].path < docs[j].path })

	cache := map[string]*doc{}
	if index != "" {
		cache = loadIndex(index)
	}
	var todo []*doc
	for _, d := range docs {
		if c, ok := cache[d.path]; ok && c.size == d.size && c.mtime == d.mtime {
			d.id, d.links, d.idrefs, d.cached = c.id, c.links, c.idrefs, true
		} else {
			todo = append(todo, d)
		}
	}
	var wg sync.WaitGroup
	ch := make(chan *doc, 256)
	var firstErr error
	var mu sync.Mutex
	for w := 0; w < runtime.NumCPU(); w++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for d := range ch {
				b, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(d.path)))
				if err != nil {
					mu.Lock()
					firstErr = err
					mu.Unlock()
					continue
				}
				// No copy: b is not modified after this point.
				parse(d, unsafe.String(unsafe.SliceData(b), len(b)))
			}
		}()
	}
	for _, d := range todo {
		ch <- d
	}
	close(ch)
	wg.Wait()
	if firstErr != nil {
		return firstErr
	}

	// resolve
	ids := map[string]int{}
	prefixes := map[string]bool{}
	var bytes, links, internal, idrefs int64
	for _, d := range docs {
		bytes += d.size
		if d.id != "" {
			ids[d.id]++
			if p := idPrefix(d.id); p != "" {
				prefixes[p] = true
			}
		}
	}
	var broken, unresolved []string
	for _, d := range docs {
		dir := path.Dir(d.path)
		if dir == "." {
			dir = ""
		}
		for _, l := range d.links {
			links++
			t, ok := internalTarget(l)
			if !ok {
				continue
			}
			internal++
			var full string
			if strings.HasPrefix(t, "/") {
				full = path.Clean(strings.TrimLeft(t, "/"))
			} else {
				full = path.Clean(path.Join(dir, t))
			}
			if full == "." {
				full = ""
			}
			if !files[full] && !dirs[full] {
				broken = append(broken, d.path+" -> "+l)
			}
		}
		for _, r := range d.idrefs {
			if !prefixes[idPrefix(r)] {
				continue
			}
			idrefs++
			if ids[r] == 0 {
				unresolved = append(unresolved, d.path+" -> "+r)
			}
		}
	}
	dups := 0
	for _, n := range ids {
		if n > 1 {
			dups++
		}
	}
	parsed := len(todo)
	if index != "" && parsed > 0 {
		if err := saveIndex(index, docs); err != nil {
			return err
		}
	}
	w := bufio.NewWriter(os.Stdout)
	defer w.Flush()
	fmt.Fprintf(w, "files_walked %d\ndocs %d\nbytes %d\nlinks %d\nlinks_internal %d\nlinks_broken %d\n",
		len(files), len(docs), bytes, links, internal, len(broken))
	fmt.Fprintf(w, "ids %d\nid_dups %d\nidrefs %d\nidrefs_unresolved %d\nparsed %d\n",
		len(ids), dups, idrefs, len(unresolved), parsed)
	if list {
		sort.Strings(broken)
		sort.Strings(unresolved)
		for _, b := range broken {
			fmt.Fprintln(w, "broken", b)
		}
		for _, u := range unresolved {
			fmt.Fprintln(w, "unresolved", u)
		}
	}
	return nil
}

// parse fills id, links and idrefs from Markdown text.
func parse(d *doc, s string) {
	lines := strings.Split(s, "\n")
	start := 0
	if len(lines) > 0 && strings.TrimRight(lines[0], "\r") == "---" {
		for j := 1; j < len(lines); j++ {
			l := strings.TrimRight(lines[j], "\r")
			if l == "---" || l == "..." {
				for _, fl := range lines[1:j] {
					k, v, ok := strings.Cut(fl, ":")
					if ok && k == "id" {
						d.id = strings.Trim(strings.TrimSpace(v), "\"'")
					}
				}
				start = j + 1
				break
			}
		}
	}
	if d.id == "" {
		d.id = leadingID(path.Base(d.path))
	}
	fenceCh, fenceN := byte(0), 0
	for _, raw := range lines[start:] {
		l := strings.TrimRight(raw, "\r")
		if ch, n := fence(l); n > 0 {
			if fenceN == 0 {
				fenceCh, fenceN = ch, n
				continue
			}
			if ch == fenceCh && n >= fenceN {
				fenceN = 0
				continue
			}
		}
		if fenceN > 0 {
			continue
		}
		l = stripCode(l)
		links, prose := findLinks(l)
		// Clone: a substring would keep the whole file buffer alive.
		for _, x := range links {
			d.links = append(d.links, strings.Clone(x))
		}
		for _, x := range findIDs(prose) {
			d.idrefs = append(d.idrefs, strings.Clone(x))
		}
	}
}

// fence returns the fence char and run length when l opens or closes a fence.
func fence(l string) (byte, int) {
	i := 0
	for i < len(l) && i < 3 && l[i] == ' ' {
		i++
	}
	if i >= len(l) || (l[i] != '`' && l[i] != '~') {
		return 0, 0
	}
	ch := l[i]
	n := 0
	for i+n < len(l) && l[i+n] == ch {
		n++
	}
	if n < 3 {
		return 0, 0
	}
	return ch, n
}

// stripCode removes inline code spans (matched backtick runs).
func stripCode(l string) string {
	if strings.IndexByte(l, '`') < 0 {
		return l
	}
	var b strings.Builder
	i := 0
	for i < len(l) {
		if l[i] != '`' {
			b.WriteByte(l[i])
			i++
			continue
		}
		n := 0
		for i+n < len(l) && l[i+n] == '`' {
			n++
		}
		end := -1
		for j := i + n; j < len(l); {
			if l[j] != '`' {
				j++
				continue
			}
			m := 0
			for j+m < len(l) && l[j+m] == '`' {
				m++
			}
			if m == n {
				end = j + m
				break
			}
			j += m
		}
		if end < 0 {
			b.WriteString(l[i : i+n])
			i += n
			continue
		}
		i = end
	}
	return b.String()
}

// findLinks returns inline link targets (the text after "](" up to ")" or a
// space) and the line with those targets removed, for the id scan.
func findLinks(l string) ([]string, string) {
	var out []string
	var prose strings.Builder
	for {
		k := strings.Index(l, "](")
		if k < 0 {
			prose.WriteString(l)
			return out, prose.String()
		}
		prose.WriteString(l[:k+2])
		rest := l[k+2:]
		var t string
		if strings.HasPrefix(rest, "<") {
			e := strings.IndexByte(rest, '>')
			if e < 0 {
				l = rest
				continue
			}
			t, l = rest[1:e], rest[e+1:]
		} else {
			e := strings.IndexAny(rest, ") \t")
			if e < 0 {
				e = len(rest)
			}
			t, l = rest[:e], rest[e:]
		}
		if t != "" {
			out = append(out, t)
		}
	}
}

func isUpper(c byte) bool { return c >= 'A' && c <= 'Z' }
func isDigit(c byte) bool { return c >= '0' && c <= '9' }
func isWord(c byte) bool {
	return isUpper(c) || isDigit(c) || (c >= 'a' && c <= 'z') || c == '_' || c == '-'
}

// matchID reports the length of an id token PREFIX-NUM at s[i:], or 0.
// PREFIX: an upper-case letter, then 1-9 upper-case letters or digits. NUM: 1-6 digits.
func matchID(s string, i int) int {
	j := i
	if j >= len(s) || !isUpper(s[j]) {
		return 0
	}
	j++
	for j < len(s) && j-i < 10 && (isUpper(s[j]) || isDigit(s[j])) {
		j++
	}
	if j-i < 2 || j >= len(s) || s[j] != '-' {
		return 0
	}
	j++
	k := j
	for j < len(s) && j-k < 7 && isDigit(s[j]) {
		j++
	}
	if j == k || j-k > 6 {
		return 0
	}
	if j < len(s) && (isWord(s[j]) && s[j] != '-') {
		return 0
	}
	return j - i
}

func findIDs(l string) []string {
	var out []string
	for i := 0; i < len(l); i++ {
		if !isUpper(l[i]) || (i > 0 && isWord(l[i-1])) {
			continue
		}
		if n := matchID(l, i); n > 0 {
			out = append(out, l[i:i+n])
			i += n - 1
		}
	}
	return out
}

func leadingID(name string) string {
	if n := matchID(name, 0); n > 0 {
		return name[:n]
	}
	return ""
}

func idPrefix(id string) string {
	if k := strings.IndexByte(id, '-'); k > 0 {
		return id[:k]
	}
	return ""
}

// internalTarget strips fragment and query and decodes %XX; ok is false for
// external links and same-file anchors.
func internalTarget(t string) (string, bool) {
	if strings.HasPrefix(t, "#") {
		return "", false
	}
	for i := 0; i < len(t); i++ {
		c := t[i]
		if c == ':' && i > 0 {
			return "", false
		}
		if !(isUpper(c) || (c >= 'a' && c <= 'z') || (i > 0 && (isDigit(c) || c == '+' || c == '.' || c == '-'))) {
			break
		}
	}
	if k := strings.IndexAny(t, "#?"); k >= 0 {
		t = t[:k]
	}
	if t == "" {
		return "", false
	}
	if strings.IndexByte(t, '%') >= 0 {
		var b strings.Builder
		for i := 0; i < len(t); i++ {
			if t[i] == '%' && i+2 < len(t) {
				if v, err := strconv.ParseUint(t[i+1:i+3], 16, 8); err == nil {
					b.WriteByte(byte(v))
					i += 2
					continue
				}
			}
			b.WriteByte(t[i])
		}
		t = b.String()
	}
	return t, true
}

func loadIndex(p string) map[string]*doc {
	out := map[string]*doc{}
	b, err := os.ReadFile(p)
	if err != nil {
		return out
	}
	lines := strings.Split(string(b), "\n")
	if len(lines) == 0 || lines[0] != indexMagic {
		return out
	}
	for _, l := range lines[1:] {
		f := strings.Split(l, "\t")
		if len(f) != 6 {
			continue
		}
		size, err1 := strconv.ParseInt(f[1], 10, 64)
		mt, err2 := strconv.ParseInt(f[2], 10, 64)
		if err1 != nil || err2 != nil {
			continue
		}
		out[f[0]] = &doc{path: f[0], size: size, mtime: mt, id: f[3], links: splitList(f[4]), idrefs: splitList(f[5])}
	}
	return out
}

func splitList(s string) []string {
	if s == "" {
		return nil
	}
	return strings.Split(s, "\x1f")
}

func clean(s string) bool { return !strings.ContainsAny(s, "\t\n\x1f") }

func saveIndex(p string, docs []*doc) error {
	tmp := p + ".tmp"
	f, err := os.Create(tmp)
	if err != nil {
		return err
	}
	w := bufio.NewWriter(f)
	fmt.Fprintln(w, indexMagic)
	for _, d := range docs {
		ok := clean(d.path) && clean(d.id)
		for _, x := range d.links {
			ok = ok && clean(x)
		}
		if !ok {
			continue
		}
		fmt.Fprintf(w, "%s\t%d\t%d\t%s\t%s\t%s\n", d.path, d.size, d.mtime, d.id,
			strings.Join(d.links, "\x1f"), strings.Join(d.idrefs, "\x1f"))
	}
	if err := w.Flush(); err != nil {
		f.Close()
		return err
	}
	if err := f.Close(); err != nil {
		return err
	}
	return os.Rename(tmp, p)
}
