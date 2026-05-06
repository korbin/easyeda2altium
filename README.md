# easyeda2altium

Convert LCSC/EasyEDA components into Altium `.SchLib` and `.PcbLib` files.

## Build

```
cargo build --release
```

## Usage

```
easyeda2altium [OPTIONS] --lcsc_id <LCSC_ID>...
```

Options:

```
--lcsc_id <ID>...     One or more LCSC IDs, e.g. C2040 C25744.
--symbol              Emit .SchLib.
--footprint           Emit .PcbLib.
--3d                  Embed STEP model in .PcbLib's Library/Models.
--full                Equivalent to --symbol --footprint --3d.
--output <PATH>       Output basename; produces <PATH>.SchLib and <PATH>.PcbLib.
                      Defaults to ~/Documents/easyeda2altium/easyeda2altium.
--overwrite           Replace existing output files.
--use-cache           Cache API responses under ./.easyeda_cache/.
--custom-field K:V... Add a hidden RECORD=41 parameter to each symbol.
--strip-chinese       Drop CJK ideographs (and surrounding parens) from strings.
--font <NAME>         Font for all text in the output.
```

## Examples

Single component, all artifacts, custom font:

```
easyeda2altium --lcsc_id C2040 --full --output /tmp/rp2040 --font 'Source Code Pro'
```

Multiple components into one library pair:

```
easyeda2altium --lcsc_id C2040 C25744 C2685 --full --output /tmp/lib
```

Symbol only, with extra parameters:

```
easyeda2altium --lcsc_id C2040 --symbol --output /tmp/rp2040 \
  --custom-field 'Manufacturer:TI' 'Tolerance:1%'
```

## Cache

`--use-cache` stores API responses under `./.easyeda_cache/<lcsc_id>.json` and
STEP bodies under `./.easyeda_cache/<uuid>.step`. Subsequent runs skip the
network for cached IDs. Delete the directory to force a refresh.

## Output

`--output /path/base` produces `/path/base.SchLib` and `/path/base.PcbLib`.
Both are OLE compound files that load directly in Altium Designer.

## Dependencies

- [`altium`](https://github.com/korbin/altiumrs): Altium file format I/O.
- `clap`: argument parsing.
- `reqwest` (rustls): EasyEDA API client.
- `tokio`: async runtime.
- `serde` / `serde_json`: JSON parsing.
- `thiserror`: error types.

## License

Dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
