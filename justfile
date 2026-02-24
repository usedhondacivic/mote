# Firware recipes
mod firmware './mote-firmware'
# API recipes
mod api './mote-api'
# Documentation book recipes
mod book './mote-book'
# Configuration website recipes
mod config './mote-configuration'
# KiCAD circuit design recipes
mod hardware './mote-hardware'

[default]
_default:
    just --list

# Run the full CI suite
ci: firmware::ci api::ci book::ci config::ci

# Generate a folder for uploading to gh pages
ci-web-artifact: book::build config::ci-build
    mkdir -p output/configuration
    cp -r mote-book/book/* output
    cp -r mote-configuration/dist/* output/configuration

