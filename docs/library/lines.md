# Lines: patterns, weights and styles

A drawing standard (`bim_library::drawing::Standard`) keeps three things of a line apart, as
Revit and AutoCAD do. This says what each is, what it is in those two programs, and where the
starter values come from.

| Here | Revit | AutoCAD | What it is |
|---|---|---|---|
| Line pattern (`LinePattern`) | Line pattern | Linetype (`.lin`) | Dashes, gaps and dots |
| Line weight (`Pen`) | Line weight (pen 1 to 16, a width per scale) | Lineweight (a fixed list in mm) | A width on paper |
| Line style (`LineStyle`) | Line style (weight, colour, pattern) | What a layer gives a line (colour, linetype, lineweight) | A weight, a pattern and a colour under a name |

## Line patterns

- A pattern is dash, gap, dash, gap, ... as **multiples of the line width**, so a pattern grows
  with the pen it is drawn with, as ISO 128-2 has it. Revit and AutoCAD give lengths in
  millimetres instead; `LinePattern::lengths(width)` gives those.
- A dot is a dash of half a line width (`DOT`).
- The standard starts with the fifteen line types of ISO 128-2, numbered 1 to 15
  (`LinePattern::number`): dots 0.5, gaps 3, short dashes 6, dashes 12, long dashes 24 and wide
  gaps 18 line widths. The lengths are the ones AutoCAD's `ACAD_ISO02W100` to `ACAD_ISO15W100`
  have; they were written from memory and not checked against the text of the standard.
- **AutoCAD linetypes.** `from_lin(text, width)` reads a `.lin` file, `lin(patterns, width)`
  writes one. Lengths in the file are millimetres at a linetype scale of 1; `width` (0.25 mm
  when none is said, `LIN_WIDTH`) turns them into line widths and back. A dot is `0` in the
  file. Left out when reading: a linetype with text or a shape in it (`[...]`), one without a
  gap, and one whose dashes and gaps do not pair. A continuous line is not written: AutoCAD has
  its own.
- In an IFCX file a pattern is the attribute `openaec::drawing::linestyle`. The name is from
  before line styles existed and is kept so files written earlier still read.

## Line weights

- The pens are the ISO series: 0.13, 0.18, 0.25, 0.35, 0.5, 0.7, 1, 1.4 and 2 mm.
- A pen has one width, and where an office wants it another width at a scale (`Pen::at`,
  `Pen::width_at(scale)`), as Revit's model line weights have a width per scale. A scale that
  has none takes the width of the nearest scale that has one, by ratio. The starter pens have
  one width at every scale: a kind of drawing picks a heavier pen at a larger scale instead.
- `Standard::nearest_pen(width)` gives the pen for a width from elsewhere (a DXF lineweight, a
  Revit line weight).

## Line styles

A line style names a pen and a pattern, and a colour where it has one; without one a line has
the colour of what it is drawn for. `Standard::drawn(key, scale)` gives the line as it is drawn:
its width at that scale, its dashes in millimetres and its colour. In an IFCX file it is the
attribute `openaec::drawing::line`.

| Key | Dutch | Pen | Pattern | From |
|---|---|---|---|---|
| `thin` | dunne lijn | 0.18 | continuous | Revit's Thin Lines; ISO 128 narrow line |
| `medium` | middeldikke lijn | 0.35 | continuous | Revit's Medium Lines |
| `wide` | dikke lijn | 0.5 | continuous | Revit's Wide Lines; ISO 128 wide line |
| `extra-wide` | extra dikke lijn | 0.7 | continuous | ISO 128 extra wide line |
| `hidden` | verborgen lijn | 0.18 | dashed | ISO 128 type 02, hidden edges; Revit's Hidden Lines |
| `overhead` | boven het snijvlak | 0.18 | long-dashed double-dotted | ISO 128 type 05, parts in front of the cutting plane; Revit's Overhead |
| `centre` | hartlijn | 0.13 | long-dashed dotted | ISO 128 type 04 narrow; Revit's Centerline |
| `grid` | stramienlijn | 0.18 | long-dashed dotted | usual for grids |
| `cutting-plane` | snijlijn | 0.5 | long-dashed dotted | ISO 128 type 04 wide, cutting planes |
| `demolished` | te slopen | 0.18 | dashed spaced | Revit's Demolished; the pattern is the suite's choice |
| `boundary` | erfgrens | 0.35 | long-dashed double-short-dashed | AutoCAD's PHANTOM, usual for plot boundaries |

The table is a proposal. The uses of the ISO line types and the names of Revit's and AutoCAD's
styles were written from memory; neither a Revit template nor NEN's drawing standards were read
for it. An office changes every row in the Library product.

## What a drawing draws with

A kind of drawing (`DrawingKind::lines`) gives a line a line style, by what the line is drawn
for (`Line`):

| Line | What it is | Without a style |
|---|---|---|
| `cut` | Where the plane of a plan or section cuts the model | The kind's cut pen, continuous |
| `beyond` | What is seen beyond the cut | The kind's beyond pen, continuous |
| `hidden` | What a structural plan sees through the floor | The kind's hidden line pattern, as wide as what is seen beyond |
| `grid` | Grid lines and levels, with their bubbles | 0.18 mm, dashes of 8 and 0.35 mm with gaps of 1.5 mm |
| `section` | The line of a section on a floor plan, with its arrows | 0.35 mm, the same dashes |

- `Standard::stroke(kind, line, scale)` gives the line as it is drawn: the width its pen has at
  that scale, its dashes in millimetres on paper and its colour.
- The scale is the one the view is drawn at: on a sheet the scale it is placed at, by itself
  that of the first sheet it is on, else its own. A pen with another width at a scale
  (`Pen::at`) so draws the same view narrower at 1:200 than at 1:50.
- Every exported drawing (DWG, DXF, IFCCAD, PDF, PNG, SVG) and the sheet's tab draw that way;
  a view's own tab draws its grid lines and levels that way.
- Dashes are laid run by run: lines that meet end to end carry the pattern on round the corner.
- The starter kinds give `hidden` the style `hidden`, `grid` the style `grid` and `section`
  the style `cutting-plane`. What is cut and what is seen beyond stay with the pens the kind
  picks for its scale (0.7 and 0.35 up to 1:20, 0.5 and 0.25 up to 1:50, 0.35 and 0.18 beyond).
- A file written before has kinds without line styles and draws as it did, with one change: a
  section's line on an exported sheet is now the chain line the sheet's tab always showed.
- A line style a kind draws with cannot be removed from the standard.

## Not yet

- A view's own tab draws what is cut and seen as the 3D view does, not by the kind's styles.
- The sheet's tab does not draw hidden lines at all (as before); an exported drawing does.
- Lines a view's filters give a pen of their own take the width of the kind's line, not its
  dashes; hatches, dimensions, texts and detail lines have no line style of the kind.
- A view template names pens, not line styles.
- Revit's object styles (a weight, a colour and a pattern per category, cut and projected) are
  not here.
- The readers of Revit and DXF files do not map their line patterns and weights onto the
  standard's.
- Linetypes with text or shapes in them (AutoCAD's `GAS_LINE`, `FENCELINE`) cannot be held.
