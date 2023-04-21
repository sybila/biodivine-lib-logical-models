# Biodivine logical models library

**This is a work in progress. At the moment, there is practically no functionality implemented yet.**

A Rust library for working with logical models (Boolean/multi-valued networks) in systems biology.

### Goals

- [ ] Can load/store file formats common in systems biology (sbml, bnet, aeon, bma).
- [ ] Can perform basic static analysis on such models (unused variables,  invalid regulations, input inlining or general reduction).
- [ ] Can represent unknown/uncertain behaviour within the logical model.
- [ ] Can represent and manipulate the state-transition graph of a logical model symbolically (maybe using multiple different encodings?).
- [ ] Provides some basic utility algorithms for (a) exploring the structural properties of the model (feedback vertex sets, cycles, etc.) (b) exploring the model dynamics (reachability, fixed-points, trap spaces, etc.).
