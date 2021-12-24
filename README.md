Rustインストール済みであれば
```bash
cargo run --release
```
で14文字のパスワード解析が走るようになっています。
パスワードのprefixやsuffixを指定したい場合は、下記のように指定してください。
```bash
cargo run --release -- --prefix ABC --suffix XYZ
```

