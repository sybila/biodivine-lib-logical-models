[![Crates.io](https://img.shields.io/crates/v/biodivine-lib-logical-models?style=flat-square)](https://crates.io/crates/biodivine-lib-logical-models) 
[![Api Docs](https://img.shields.io/badge/docs-api-yellowgreen?style=flat-square)](https://docs.rs/biodivine-lib-logical-models/) 
[![Continuous integration](https://img.shields.io/github/actions/workflow/status/sybila/biodivine-lib-logical-models/build.yml?branch=master&style=flat-square)](https://github.com/sybila/biodivine-lib-logical-models/actions?query=workflow%3Abuild)
[![Coverage](https://img.shields.io/codecov/c/github/sybila/biodivine-lib-logical-models?style=flat-square)](https://codecov.io/gh/sybila/biodivine-lib-logical-models) 
[![GitHub issues](https://img.shields.io/github/issues/sybila/biodivine-lib-logical-models?style=flat-square)](https://github.com/sybila/biodivine-lib-logical-models/issues) 
[![Dev Docs](https://img.shields.io/badge/docs-dev-orange?style=flat-square)](https://biodivine.fi.muni.cz/docs/biodivine-lib-logical-models/latest/) 
[![GitHub last commit](https://img.shields.io/github/last-commit/sybila/biodivine-lib-logical-models?style=flat-square)](https://github.com/sybila/biodivine-lib-logical-models/commits/master) 
[![Crates.io](https://img.shields.io/crates/l/biodivine-lib-logical-models?style=flat-square)](https://github.com/sybila/biodivine-lib-logical-models/blob/master/LICENSE)


# Biodivine logical models library

**This is a work in progress. At the moment, there is practically no functionality implemented yet.**

A Rust library for working with logical models (Boolean/multi-valued networks) in systems biology.

### Goals

- [ ] Can load/store file formats common in systems biology (sbml, bnet, aeon, bma).
- [ ] Can perform basic static analysis on such models (unused variables,  invalid regulations, input inlining or general reduction).
- [ ] Can represent unknown/uncertain behaviour within the logical model.
- [ ] Can represent and manipulate the state-transition graph of a logical model symbolically (maybe using multiple different encodings?).
- [ ] Provides some basic utility algorithms for (a) exploring the structural properties of the model (feedback vertex sets, cycles, etc.) (b) exploring the model dynamics (reachability, fixed-points, trap spaces, etc.).


### Running reachability integration test

To test the correctness of the implementation, we now have a simple reachability consistency check.
This check compares the results of all symbolic representations on a basic reachability exploration.
To run the test, execute the following:

```bash
# First argument is a timeout, here 1 minute. This uses standard unix `timeout`.
# Second argument is fwd/bwd to indicate which reachability direction you want to test.
python3 reachability_integration_test.py 1m fwd
```

The process dumps all results into an appropriate `./data/results-*` directory.
The script will not overwrite existing results, 
so you have to delete the directory manually before running the test again.

You should see an output similar to the following:

```
[PASS] No error discovered in `146_BUDDING-YEAST-FAURE-2009.sbml` in less than 1m.
[PASS] No error discovered in `148_AGS-cell-fate-decision.sbml` in less than 1m.
[PASS] No error discovered in `151_TCR-REDOX-METABOLISM.sbml` in less than 1m.
[PASS] No error discovered in `155_CONTROL-OF-TH1-TH2-TH17-TREG-DIFFERENTATION.sbml` in less than 1m.
[PASS] No error discovered in `157_CONTROL-OF-TH-DIFFERENTATION.sbml` in less than 1m.
...
```

If you see a `[FAIL]` somewhere, it means an inconsistency has been detected.
You can then check the results to examine the specific case.