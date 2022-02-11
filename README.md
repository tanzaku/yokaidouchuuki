14文字のパスワードを全列挙する [ブランチ](https://github.com/tanzaku/yokaidouchuuki/tree/password_enumeration) を作成しました。  
多くのメモリが要求されますが、パスワード列挙が高速で、かつ読みやすく書き直しています。

---

Rustインストール済みであれば
```bash
cargo run --release
```
で14文字のパスワード解析が走るようになっています。
パスワードのprefixやsuffixを指定したい場合は、下記のように指定してください。
```bash
cargo run --release -- --prefix ABC --suffix XYZ
```

