# fileserver

A small function that serves the files of a site. `read_file(base, name)` gets the site folder `base` and the name of a file in it, from the URL of a request.

Run the tests with `python3 -m unittest discover -s tests -t .`.

## Site folders

- A name is relative to the site folder. A request never names a file by an absolute path, so `read_file` gives `AccessDenied` for an absolute name.
- A site folder holds files and folders. It can also hold links. For example, `latest.txt` is a link to the newest file of the same folder, and `shared` is a link to a folder of another site.
- A link that points to a file or a folder inside the site folder is fine: the function serves the files behind it.
- A file behind a link that points outside the site folder, such as `shared`, is not a file of this site. The function does not serve it.
- The path `base` can have a link in it. The function serves the same files for it.
- The name of a request comes from a stranger. The site folders sit in a folder that holds other data too.
