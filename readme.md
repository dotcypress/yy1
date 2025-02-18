# YY1

Utility to convert KiCad centroid files into Neoden YY1 pick and place machine format

## Installation

`cargo install yy1`

## Usage

```
yy1 [OPTIONS] <INPUT> <OUTPUT>

Arguments:
  <INPUT>   KiCad placement file
  <OUTPUT>  Output file(s)

Options:
  -r, --rename <package_map>    Package rename file
  -f, --feeder <feeder_config>  Feeder config file
  -n, --nozzle <nozzle_config>  Nozzle config file
  -o, --offset <offset>         PCB offset (x:y)
  -p, --panel <panel>           Panel config (rows:columns:width:length)
  -e, --explode                 Explode panel
  -b, --bom                     Generate BOM
  -s, --skip <skip>             Skip until component number
      --fiducial <fiducial>     Fiducial designator or position
  -h, --help                    Print help
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
