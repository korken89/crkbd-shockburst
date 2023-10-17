# Firmware for the CherryBurst/ChocBurst keyboard

Firmware for the CherryBurst/ChocBurst keyboard and associated dongle.

# Testing commands

Dongle radio side:

`DEFMT_LOG=info cargo rrb dongle --features dongle_radio -- --probe 1209:4853:dc61cd078f594c37ef4014 --no-location`

Keyboard radio side:

`DEFMT_LOG=info cargo rrb dongle --features keyboard_radio -- --probe 1209:4853:dc61cd078f667031ef4014 --no-location`


## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
