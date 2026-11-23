# The stock of the Dutch library

What the libraries of the suite hold today, and what an office that draws a Dutch house or a
small utility building still misses. Written for issue 307. The numbers are counted by
`bim_library::ils::stock` (run `cargo test -p bim-library print_the_stock -- --ignored
--nocapture` to print them again).

## What the libraries hold

| | General library | Dutch library |
|---|---|---|
| Materials | 93 | 132 |
| Wall build-ups | 21 | 63 |
| Floor build-ups | 13 | 48 |
| Roof build-ups | 4 | 7 |
| Ceiling build-ups | 2 | 0 |
| Columns | 11 | 24 |
| Beams | 12 | 42 |
| Piles | 0 | 2 |
| Members, mullions, skirting boards | 2 | 0 |
| System types (frames, stud walls, curtain walls, ceilings) | 38 | 0 |
| Object types (doors, windows, fittings, terminals) | 21 | 0 |
| Connection types | 25 | 0 |
| Service types and their covers | 10 and 12 | 0 |
| 2D components of Ocondat | 0 | 1,307 (1,178 with their drawing) |

The Dutch library is the INB template's types, the types of Project Ocondat and the usual
layered build-ups, with their own names. The general library is the suite's own start.

## What a delivery still lacks (BIM basis ILS)

Of every build-up, linear type and system type, in both libraries:

| | Has it |
|---|---|
| The right IFC class and predefined type | all, by rule from the type's role |
| A name | all, by rule from the layers or the profile, in Dutch and English |
| A material | all |
| An NL-SfB code | all, of two digits by rule; **none has four digits** |
| Whether it bears | columns, beams and piles by their role, and most system types by their group; **no build-up says so** (a code of two digits does not say it), nor a timber frame, a steel frame or masonry |
| Whether it is external | all, from the code (21 and 27 are external) |
| A fire rating | **none** |

So no type of either library is complete for a delivery yet. The editors of a build-up, a linear
type and a system type now show what each lacks, and take the three properties; the editor of a
door, a window, a stair and a railing takes whether it is external and its fire rating.

## What a Dutch house and a small utility building still miss

- **Ceilings** in the Dutch library: none (the general library has two).
- **Roofs**: seven build-ups (warm flat roofs and one pitched roof); the tiles and battens of a
  pitched roof are not among their layers, and there is no green roof.
- **Doors, windows and frames**: the general library has object types and frame systems; the
  Dutch library has none of its own (no usual Dutch door heights and frame sections).
- **Stairs**: object types have a group for them in the general library; the Dutch library has
  none of its own, and a stair is an object of a size, not a stair worked out to its steps.
- **Timber profiles**: the usual Dutch sawn sizes are among the linear types of Ocondat; rafters,
  purlins and battens as members with their roles are not.
- **Codes of four digits and fire ratings**: on nothing yet. They are an office's to give; a rule
  could propose the third and fourth digit from a build-up's layers.
- **Property sets**: only `LoadBearing`, `IsExternal` and `FireRating` are written; a Dutch
  delivery often also asks `Reference` and, for doors and windows, their sizes.

## Sources

Every part taken over names its source and its licence in its data file: Project Ocondat
(LGPL-3.0) for materials, types, timber classes, steel profiles and 2D components; the INB
template; NEN 47 for the hatches. The issue names another open source whose name did not come
through; it is still to be asked.
