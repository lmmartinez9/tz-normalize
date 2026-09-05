# tznorm

Timestamps that come out of logs, CSV exports, or hand-typed config files
rarely agree on a format. Some use `/` for dates, some drop the leading
zero on the month, some write the offset as `-0500`, others as `-05:00`
or just `Z`. Sorting or diffing these lines is miserable because the same
instant can be spelled a dozen ways.

`tznorm` reads timestamp-like lines and rewrites each one into a single
consistent shape: `YYYY-MM-DDTHH:MM:SS+HH:MM` (or `Z` for UTC). It does not
touch anything else on the line's neighbors — it's a line-at-a-time
formatter, meant to sit in a pipeline.

## What it currently understands

- Dates: `YYYY-MM-DD` or `YYYY/MM/DD`, with or without zero-padding
  (`2024-1-5` and `2024-01-05` both work)
- Dates with a month name, in either order (`Jan 5 2024`,
  `5 January 2024`, `January 5, 2024`), full names or three-letter
  abbreviations, case-insensitive
- Date/time separator: `T` or one or more spaces
- Time: `HH:MM` or `HH:MM:SS`, either 24-hour or 12-hour with an `am`/`pm`
  marker (`am`, `PM`, `a.m.`, `P.M.`, with or without a space before it)
- Offsets: `Z`, `z`, `+HH:MM`, `-HHMM`, `+HH`, or any mix of those shapes
- Common zone abbreviations (`EST`, `PST`, `CET`, `JST`, `IST`, ...),
  looked up in a fixed offset table and rewritten as a numeric offset
- A handful of common IANA zone names (`America/New_York`,
  `Europe/Paris`, `Asia/Kolkata`, ...), matched case-insensitively against
  the same kind of fixed offset table

Abbreviations and IANA names are both matched against a single fixed offset
each, with no DST rules and no attempt to disambiguate names that mean
different things in different places (`IST` is read as India Standard Time,
for example). A zone name that observes daylight saving reads as its
standard-time offset year-round; there's no calendar of DST transition
dates behind it. Only the zone names in the built-in table are recognized —
anything else is reported as an unrecognized offset.

Month-name dates only recognize a plain day number (no "5th" or "05th"),
and check that the day falls in `1..=31` the same way numeric dates do —
they don't validate against the actual length of the given month.

## Usage

Build and run with cargo, no other tooling required:

```sh
cargo run --release -- data.txt
```

Multiple files are processed in order, each written to stdout:

```sh
cargo run --release -- jan.log feb.log
```

Reading from stdin works with no arguments, or with `-` mixed in among
file arguments:

```sh
cat data.txt | cargo run --release
tail -f app.log | cargo run --release -- -
```

Pass `--to-utc` to shift every timestamp that carries a known, non-zero
offset (numeric or a recognized zone abbreviation) so it reads as `Z`
instead. Lines with no time, or a time with no offset at all, have nothing
to shift by and pass through unchanged:

```sh
cargo run --release -- --to-utc data.txt
```

```
2024-01-05T09:30:00-05:00  ->  2024-01-05T14:30:00Z
2024-01-05T23:30:00 PST    ->  2024-01-06T07:30:00Z
2024-01-05                 ->  2024-01-05
```

Pass `--output <path>` (or `--output=<path>`) to write normalized lines to a
file instead of stdout. Error and warning messages still go to stderr either
way:

```sh
cargo run --release -- --output normalized.txt data.txt
```

### Example

Input:

```
2024-1-5T09:30:00-0500
2024/01/05 09:30:00 -05:00
2024-01-05T14:30:00Z
```

Output:

```
2024-01-05T09:30:00-05:00
2024-01-05T09:30:00-05:00
2024-01-05T14:30:00Z
```

Lines that don't match a recognized shape are reported on stderr with
their source and line number, and don't stop the rest of the input from
being processed. If any line failed to parse, the process exits with
status 1.

## Roadmap

- [x] Recognize common zone abbreviations (EST, PST, CET, ...) with a
      fixed offset table
- [x] Parse month-name dates (`Jan 5 2024`, `5 January 2024`)
- [x] Add a `--to-utc` flag that converts every offset to `Z`
- [x] Support 12-hour clock times with am/pm
- [x] Add an `--output` flag to write to a file instead of stdout
- [x] Support IANA zone names like `America/New_York`
