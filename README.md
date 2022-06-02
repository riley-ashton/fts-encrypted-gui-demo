# fts-encrypted-gui-demo
A demo of my fts-encrypted library using dioxus and the public enron email dataset.
The main purpose of this library is just as a proof of concept for testing
search speed in a common client side scenario.

The main library for [fts-encrypted is here](https://github.com/riley-ashton/fts-encrypted)

## Using

```sh
git clone https://github.com/riley-ashton/fts-encrypted-gui-demo
cd fts-encrypted-gui-demo
cargo run --release
```

## TODO
- the actual emails themselves are not encrypted, only the full text search indicies are. 
Add AES-CBC, AES-GCM or similar to encrypt the emails?
