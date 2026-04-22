#!/bin/bash

# Generate/update license info for all dependencies
# Re-run this any time dependencies are modified
cargo lichking bundle > attribution.txt