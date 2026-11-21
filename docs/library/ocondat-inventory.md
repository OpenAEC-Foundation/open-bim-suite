# Project Ocondat: what there is, and what the library takes

An inventory of `OpenAEC-Foundation/project-ocondat` (LGPL-3.0) as it stood at commit
`85a78eef1`, made for issue 311. The counts were taken from a checkout by walking its files
and hashing every DXF; they are not from the repository's own README.

## What there is

| Where | What | Format | Count | State |
| --- | --- | --- | --- | --- |
| `DXF Library/components/` | 2D components by maker and series, each in a folder with a page | DXF, SVG, Markdown | 22,835 DXF | the ordered set; has an index |
| `DXF Library/component-library/` | a web page over the components, its index and their previews | `components.json` (24 MB), SVG | 22,833 entries | the index every import should start from |
| `DXF Library/Downloads/` | what the components were made from, as downloaded | DXF, DWG, ZIP | 21,591 DXF | double: see below |
| `DXF Library/Componenten/` | the generic components in their first folders, by NL-SfB | DXF | 1,477 | double with `components/Generic` |
| `DXF Library/001` to `004` | floors, aluminium frames, a sill, sandwich panels | DXF, DWG | 86 DXF | double with `Componenten` |
| `DXF Library/Nieuwe map/`, two loose files | work in progress, one file of 29 MB with all components | DXF | 26 | not sorted, not taken |
| `BlenderBIM/Library.xlsx` | materials, profiles, single-layer walls and floors | Excel | 1 | taken (see below) |
| `BlenderBIM/IFC Arceringen.ifc`, `IFC4 NL Demo Template 0.2.ifc` | hatches and a template | IFC | 2 | not taken: the hatches come from the INB template |
| `steelprofile.json`, `Data/Steelprofiles.xlsx` | the steel table | JSON, Excel | 2 | taken |
| `Standards/Eurocode` | wind loads by NEN-EN 1991-1-4 with the Dutch annex | ODS, Python | 2 files | not taken: loads are not library content |
| `Data/Brondata` | the scans the steel and reinforcement tables were made from | PDF, PNG | 7 files | not taken: sources, not data |
| `Libraries/Netherlands` | pages for building, structure, services, civil work, reinforcement and piles, with the INB template's source | HTML, JS, Python, SVG | 1,799 files | the timber strength classes of `data/hout.py` are taken; the rest is not: the INB template is read from its own repository |
| `Revit/` | two templates and structural families | RTE, RFA | 30 files | not taken: no reader for them here |

By format, the whole checkout: 47,666 DXF (with copies and backups), 27,181 Markdown pages,
24,422 SVG, 484 DWG, 28 RFA, 26 ZIP.

## What is double

Of 46,015 DXF files under `DXF Library`, 23,035 differ in content. Half the folder is a
second copy:

- `Downloads/Halfen` and `components/Halfen`: 18,604 files, the same in both.
- `Downloads/DXF-library` and `Downloads/dxfBlocks` are `components/Community`: 2,560.
- `Downloads/ArcelorMittal` is `components/ArcelorMittal`: 359.
- `Componenten` is `components/Generic`: 1,305 of its 1,307, and the numbered folders
  `001` to `004` are in both again.

So `components/` with its index is the one set to import from; the rest is where it came from.

## The components, by who made them

| Maker | Entries | Licence, as Ocondat states it | Taken |
| --- | --- | --- | --- |
| Generic | 1,307 | LGPL-3.0 | yes |
| Halfen | 18,606 | "Manufacturer documentation - free for engineering use" | no |
| ArcelorMittal | 359 | "Manufacturer documentation - free for engineering use" | no |
| Community/DXF-library | 2,256 | "Mixed - see individual series" | no |
| Community/dxfBlocks | 305 | "Mixed - see individual series" | no |

"Free for engineering use" is not a licence to pass the files on in another product, and
"mixed" names none. Those four sets wait until their terms are read at the source; the import
counts them and leaves them out.

How good the index is:

- Every generic entry has an id of its own, a DXF that exists, an IFC class, a predefined
  type and an NL-SfB code of two digits. Two have no size.
- Outside Generic the NL-SfB code is empty for 21,526 of 21,526 entries, and 18,470 of
  Halfen's are one series (`HIT-Verbindungen`) of one class.
- The IFC class is that of the series, not of the part: every base material is an
  `IfcMechanicalFastener`, also the break lines. It is taken as it is and wants a pass by
  hand in Ocondat.
- Ocondat's own generic drawings are on the layer `A-DETL-GENF` (5,119 of 7,119 entities in a
  sample of 150 files); the rest is on `0` or on a layer named after a material.

## What the library takes, and how

| Of Ocondat | In the library | Made by |
| --- | --- | --- |
| materials of `Library.xlsx` | `data/ocondat-materials.json` | `tools/ocondat-materials/extract.py` |
| types of one material of `Library.xlsx` | `data/ocondat-types.json` | the same |
| the steel table | `data/ocondat-steel.json` | `tools/ocondat-steel` |
| the timber strength classes | `data/ocondat-timber-grades.json` | `tools/ocondat-grades` |
| the generic 2D components | `data/ocondat-components.json` | `tools/ocondat-components/extract.py` |

Each tool is run again against a checkout when Ocondat changes; nothing is copied by hand,
and each data file names its source, its commit and its licence.

A component is categorised by its series: IFC class, predefined type, NL-SfB code, kind
(a detail for the series of details, else a component) and the maker `generic`. Its layer is
`<discipline>-DETL-<NL-SfB code>`, in the form of Ocondat's own `A-DETL-GENF`: `S` for what
bears (members, piles, fasteners), `P` for pipes, `E` for cables, `M` for ducts, else `A`.
The suite has no layer standard of its own yet; `bim_library::component::Series::layer` is
the one place to change when it has.

## Not yet

- The drawings themselves: the catalogue points at a component's DXF in Ocondat. A plan, a
  section or a detail cannot place a component until its lines are read into the library.
- The components as the 2D views of a 3D element.
- The makers' and the community's components, until their terms are checked.
- The Revit families and templates, the IFC template, and the pages under `Libraries/`.
