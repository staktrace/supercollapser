# SuperCollapser

This is a tool to collapse redundant clauses and rules in the WPT meta-files
in mozilla-central.

## Usage

```
git clone https://github.com/staktrace/supercollapser
cd supercollapesr
./supercollapse /path/to/mozilla-central
```

This will make changes to the `testing/web-platform/meta` folder in your
mozilla-central folder. Diff to verify and commit.
