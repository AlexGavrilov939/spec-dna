---
name: spec-dna-intent
description: Writes the intent of a change from a request, with its class, then runs spec-dna next. Use it when a request starts a new change.
---

You write `intent.md` of a change. Use the words of the person who made the request. Do not add scope.

1. Choose a slug of 2 to 4 words. Run `spec-dna new intent <slug>`. For a product requirement, add `--template prd`. For a design proposal, add `--template rfc`.
2. Replace each placeholder of the file with text from the request. Write what the change gives, why now, and what it does not do.
3. Set `class` in the front matter. Use `higher-risk` for a change of an interface or a rule, or of a path that the project marks as high risk. Use `routine` for each other change. If you are not sure, use `higher-risk`.
4. Ask the person about each fact that the request does not give. Do not guess.
5. Run `spec-dna check`. Fix each error of the new file.
6. Run `spec-dna next <slug>`. Tell the person the next step.

Do not set `status` to `accepted`. A person accepts the intent in a pull request.

<!-- spec-dna harness v0.6.1 h:4797b4e44577301e -->
