# Levels of detail

What LOD 200, 350 and 500 mean for an item of the library, so a level means the same for every
item. Written for issue 312.

Two things are called LOD in the suite, and they are two settings:

- **How far a type is worked out** (`bim_library::lod`, LOD 200, 300, 350, 400): what may be
  relied on. A type has it by rule: a build-up of the generic material is 200, one of real
  materials 300, a system with its members 350, a frame worked out to its boards 400.
- **How much of it is shown** (`bim_library::detail`, LOD 200, 350, 500): what a view draws and
  which data is filled in. This document is about this one.

A type can be shown at a lower level than it is worked out to, never at a higher one without
making something up (`Detail::up_to`).

## The three levels

| | LOD 200 | LOD 350 | LOD 500 |
|---|---|---|---|
| **What it is** | A design element | The element as it is built | Everything there is |
| **Drawn** | Its outline and no more: plain, black | Its layers, its parts, its joins to what is next to it | Every part, fixing and finish |
| **A material** | The paper, or a fill | Its hatch | Its hatch, with all that is known of it |
| **A build-up** | One band, as thick as its layers together | Its layers, each with its hatch | Its layers, each with its hatch and its data |
| **Data of a material** | Its name | And its hatch and its grade | And its colour, its key, the values of its strength class and their source |

## What is made and what is drawn

A lower level is made from a higher one where that can be done:

- The outline of a build-up at 200 is made from its layers: their thicknesses together.
- The data at a level is the data of the level above, less what that level adds.

Where it cannot be made (the simple shape of a detailed object, a symbol for a fixture), it is
to be drawn by hand and kept with the item. That is not built yet.

## Where this comes from

The numbers are those of the BIMForum Level of Development Specification, where 200 is a generic
element of approximate size, 350 adds the interfaces to other elements and 500 is the element as
it was verified on site. The owner's use of them here is about what is shown: 500 is "everything
there is", not "checked on site". The suite does not record that an element was measured as
built.

Not yet compared with what the Dutch sector's own documents say of these levels.

## Not yet

- A view template names the level it asks (`ViewTemplate::level`: an outline is 200, one hatch
  and the layers 350, all of it 500), and its editor draws a sample wall at it. A drawing of the
  Modeller does not follow a template yet, so it shows every element as it did.
- Linear types, system types and objects are the same at every level.
- Views per level drawn by hand (3D, plan, section, elevation).
- How the kernel shows a wall at a level (issue 302, with Quint).
