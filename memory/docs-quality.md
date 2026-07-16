---
name: Documentation Quality Standards
description: Always maintain high-quality Rust docs like human-written code
type: feedback
---

# Rule

Always treat documentation as production code. The project owner's reputation is on the line - every doc comment, example, and error message must pass the "would a senior Rust developer wrote this?" test.

## Why:

Users judge the project by its documentation quality. AI-generated docs often:
- Repetitive phrasing
- Over-explaining basics
- Examples that don't compile or are unrealistic
- Missing real-world context

This erodes trust before users even try the code.

## How to apply:

1. **Read existing docs critically** - would you be embarrassed sharing this on GitHub?
2. **Examples must compile** - at minimum `compile_fail` or mark as `ignore`
3. **Error messages tell real stories** - not "invalid value" but "port must be 1-65535, got '80808'"
4. **Doc comments are API contracts** - each public item has exactly one doc comment
5. **Run `cargo test --doc`** as part of every PR

## Documentation checklist for each PR:

- [ ] All public items have doc comments (use `#[deny(missing_docs)]`)
- [ ] Examples appear in the README and compile
- [ ] Error messages include actual file/line positions where possible
- [ ] No TODOs in doc strings
- [ ] No "In real projects you would..." - just show the right way
