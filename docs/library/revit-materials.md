# The materials of a Revit library, mirrored

The owner asked which materials a Revit library usually holds, and to make those in the suite.
This says what was looked at, what the suite had, and what was added.

## What was looked at

- **The Revit template of Project Ocondat** (`Revit/Revit Template 2023.rte`, LGPL-3.0), read with
  the suite's own Revit reader: 121 materials. Run it again with
  `RVT_MATERIALS=<a Revit file> cargo test --release print_the_materials_of_a_revit_file -- --ignored --nocapture`.
- **Autodesk's own material library is not on this machine and was not read.** What it usually
  holds is taken from the materials of Revit's own that the template still has, from the classes
  Revit orders materials by, and from general knowledge of its default templates. It is a list of
  kinds, not of Autodesk's names.

## What the template holds

| Group | Count | What |
|---|---|---|
| Revit's own | 67 (55 names) | working materials: `Default`, `Default Wall`, `Default Roof`, `Poche`, the phase materials (existing, demolished, temporary), the analytical surfaces of the energy model, the render materials; and nine real ones: `Copper`, `Glass`, `Iron, Ductile`, `Polyvinyl Chloride, Rigid`, `Stainless Steel`, `Steel`, `Steel, Carbon`, `Wood` |
| The Dutch office materials | 45 | named by an NL-SfB letter and a description: `f2_beton_ihwg_C30/35`, `g2_baksteen`, `i2_hout_vuren_C18`, `m1_isolatie_steenwol`, `n7_isolatie_PIR`, `h2_Betonstaal standaard, B500`, `h3_bout_sterkeklasse_8.8` |
| A maker's | 6 | the aluminium, glass, gasket and isolator of one window system |
| Other | 3 | `Holz`, `ISI`, `NSI` |

So the template says little of what Revit's library usually holds: of Revit's own, all but
nine are working materials. The list of kinds below is from general knowledge.

By Revit's material class: 60 unassigned, 31 generic, 6 masonry, 5 wood (and 3 "Holz"), 5 concrete,
3 system, 2 stone, 3 metal and steel, 1 glass. Half of the materials have no class at all.

The Dutch office materials are those the Dutch library already takes from Ocondat's own
workbook (`bim_library::ocondat`: 109 materials with their hatches and grades); they are not
taken a second time from the template.

## What a Revit library usually holds, by kind

Revit orders materials by class: brick and masonry, concrete, stone, wood, metal, glass, plastic,
ceramic, carpet and textile, paint and coatings, insulation, gas and liquid, earth, and generic.
A model taken over has materials of those kinds, and for a site: earth, sand, gravel, asphalt,
grass and water.

## What the suite had and what was added

The general library had 38 materials: concrete and its floor slabs, brickwork, sand-lime brick,
insulation, timber and boards, steel, aluminium, zinc, copper, glass, membranes, mortars and
screed. It missed most of the finishes, the plastics, several metals and everything of a site.

**Added: 39 materials** (`bim_library::material::common`, in the general library every project
starts with):

| Kind | Materials |
|---|---|
| Masonry, stone and what is laid on them | precast concrete, concrete block, aerated concrete, natural stone, ceramic tiles, plaster |
| Timber and boards | hardwood, wood flooring, chipboard, MDF, laminate, fibre cement |
| Metals | stainless steel, cast iron, steel deck, lead, brass |
| Insulation | EPS, XPS, glass wool |
| Membranes | EPDM, bitumen, damp-proof course, sealant |
| Finishes | carpet, vinyl flooring, linoleum, rubber, paint, textile |
| Plastics and glass | plastic, polycarbonate, mirror |
| A site | earth, sand, gravel, asphalt, grass, water |

Each has a name in Dutch, English, German and French by the suite's own rule, a colour, and
where the hatch catalogue has one for its kind, that hatch (natural stone, softwood and hardwood,
boards, insulation, lead, plastic, sealant, ground, sand, gravel, water, glass).

## An office's own Revit library

After the template, the Revit library of a Dutch office was read the same way (its library
template, its model template and its details file; the files are not in the repository): 747
distinct materials. About 260 are written to the Dutch convention of a material code and a
description (`f2_beton_...`, `g2_baksteen_...`, `i2_hout_...`), many of them variants of one
kind (brick by format, colour and bond; timber by species and finish); about 90 are Revit's
working materials; the rest come with the families of makers, or are single colours.

That gave two things more:

- **Sixteen kinds more** (the general library now has 93 materials): roof tiles, reed, sedum,
  roof garden substrate, drainage layer; anhydrite screed, concrete pavers, gypsum blocks,
  polyester; clay, peat, planting; toughened and frosted glass, anodised aluminium, bamboo.
- **The material code and the match by name** (`bim_library::kind`): a material of the suite has
  its NL-SfB material code by its key (`f2` concrete, `g2` fired clay, `i2` softwood), written
  to its node as `nlsfb::material`; and `Library::material_like` says which material of the
  suite a name of another program is a kind of, by the words in it, else by the code it is
  written with. Of the 747 names it places 466; what it leaves are working materials, makers'
  materials and colours.

The codes are as files written to that convention use them. They are not checked against table
3 of NL-SfB itself.

## What is mirrored and what is not

- **The kinds are mirrored, not the content.** No name, colour, hatch or appearance asset of
  Autodesk's library is taken over.
- **Not made**: Revit's own working materials (`Default`, `Poche`, the phase and analytical
  materials, render materials). The suite has `generic` for what is not known, and draws phases
  and analytical surfaces by other means.
- **Not made**: appearance assets (textures and render settings) and the thermal and structural
  assets of a Revit material. The suite's materials take their physics and their strength class
  from the library's own fields (`bim_library::physics`, `bim_library::class`).

## Not yet

- The readers do not use the match yet: a material taken over keeps its own name and colour.
- The variants of one kind (a brick's format, colour and bond; a timber's finish) are one
  material of the suite.
- The colours are chosen to tell materials apart in a section and in 3D; they are not checked
  against a colour standard.
