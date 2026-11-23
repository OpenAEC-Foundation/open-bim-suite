# Aspects of a library item

The library is to be the one place where every aspect of a building product stands. An aspect
is a set of properties of one item with a source and a date (`bim_library::aspect`). Written for
issue 314.

## The frame

- An aspect belongs to one item (a material, a build-up, a linear, system, object, connection,
  service or cover type), named by the path of its node, and is of one kind. An item has at most
  one aspect of a kind.
- Every aspect has a **source**, a **date** (`YYYY-MM-DD`) and, where it holds in one place
  only, a **region**. Without a source and a date it is refused.
- Its properties are free: a name and a JSON value. A kind is filled in without the library
  changing its shape. A kind with a shape of its own reads its properties as that (costs:
  `bim_library::cost::Cost`).
- In a file an aspect is an attribute on the item's node, `openaec::aspect::<kind>`, written as
  a node of its own with the item's path. IFCX composes the attributes of nodes of one path, so
  a supplier's prices can be a layer over a library.

```json
{"path": "<the item's path>", "attributes": {"openaec::aspect::cost": {
  "source": "an offer", "date": "2026-03-12", "region": "NL",
  "unit": "m²", "currency": "EUR", "material": 48.0, "labour": 31.5, "equipment": 4.0,
  "index": {"name": "BDB", "value": 120.0}}}}
```

## The kinds, and what could supply each

| Kind | Attribute | What it holds | What could supply it | State |
|---|---|---|---|---|
| Costs | `openaec::aspect::cost` | A price per unit, split into material, labour, equipment, subcontractor and other; a currency; the index on its day | An office's own prices and offers; Open Calc Studio's catalogues. No open price list was found to pass on | **Built** (the shape; no prices) |
| Strength | (on the material: `openaec::strengthclass`) | The values of a strength class with their standard | The Eurocodes' formulae; the Normenatlas once it has values | **Built** (issue 313), not as an aspect: it follows from the grade |
| Environment | `openaec::aspect::environment` | Indicators per unit over the life cycle | EPDs of makers; national databases. Which of them may be passed on is **not checked** | Frame only |
| Physics | `openaec::aspect::physics` | Density, thermal conductivity, specific heat, vapour resistance factor, reaction to fire class (`bim_library::physics::Physics`) | Makers' declarations; design values of standards (to be computed or cited, not copied) | **Built** (the shape and an editor on a material; no values). Not yet: sound |
| Planning | `openaec::aspect::planning` | Production rates: hours per unit, crew | An office's own norms; Open Planner Studio | Frame only |
| Maintenance | `openaec::aspect::maintenance` | Life span, cycle of upkeep | An office's own data; standards for condition (to be cited) | Frame only |
| Maker | `openaec::aspect::maker` | Article number, GTIN, a link | The maker | Frame only |

Costs and the strength classes are the first two, as the issue asks. The order of the others is
open: it follows which source is found that may be passed on.

## Costs in Open Calc Studio's terms

So an estimate reads the library without conversion (read from `open-calc-studio@main`,
`src/types/costModel.ts` and `src/state/slices/resourceLibrarySlice.ts`):

- **Units** are its codes: `st`, `m`, `m²`, `m³`, `kg`, `ton`, `uur`, `dgn`, `ls`. Its aliases
  `m1`, `m2`, `m3` and `dag` are read.
- **Resource types** are its codes: `materiaal`, `arbeid`, `materieel`, `onderaannemer`,
  `overig`.
- **A catalogue** (`bim_library::cost::catalogue`) is the JSON its `parseLibraryJson` reads:
  `{id, version, currency, resources}`, each resource with `id`, `code`, `description`, `unit`,
  `resourceType`, `category`, `defaultUnitPrice`, `referenceUnitPrice`, `currency`, `source`,
  `priceDate`. An article there is one resource of one type, so a price split in three is three
  articles.
- **A price that is not marked verified** goes out as `referenceUnitPrice`, which Open Calc
  Studio shows and does not copy into an estimate. Its documentation asks for this.

What Open Calc Studio does not have, and the library keeps for itself: the split within one
price, the region (it writes one but does not read it) and the index.

## Not yet

- No prices and no physical values are in the library. The physics of a material are typed in
  the material editor; costs and the other kinds have no editor: they come from a file.
- VAT is not recorded; labour is a price, not hours at a rate.
- The IFCX tab's aspects (issue 216) do not show these yet.
- A type that gets another id when a library is merged leaves its aspects behind.
- The other kinds have no shape of their own and no source.
