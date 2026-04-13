# Cli

## Testing

You can test your changes to the `cli` crate by first building the main raijin binary:

```
cargo build -p raijin-app
```

And then building and running the `cli` crate with the following parameters:

```
 cargo run -p cli -- --raijin ./target/debug/raijin.exe
```
