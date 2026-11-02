# Lab diagram icons

These are **placeholders**. They must be replaced with the official AWS
Architecture Icons before this ships to anyone, per phase 3 (R3.10):

- Download the official AWS Architecture Icons toolkit from
  https://aws.amazon.com/architecture/icons/
- Copy the SVGs for the products the scenarios use into this directory,
  keeping these filenames (the asset registry in `gen-ai-core/src/assets.rs`
  maps icon keys to these names). Do not modify the icons.
- The lab must carry an AWS Architecture Icons attribution note.

The placeholders here are deliberately plain so nobody mistakes them for the
real marks: a labelled rounded square in the product's rough brand colour.
They let the diagram render and the mechanism be tested without redistributing
AWS assets from this repo. The product name is always the accessible label, so
with or without the real icons the diagram reads the same to assistive tech.
