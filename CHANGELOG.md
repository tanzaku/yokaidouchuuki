# CHANGELOG
## [0.2] - 2021/12/24
- 後ろ向き枝刈りの不具合で、真の解に到達できない可能性があった不具合を修正しました
- 後ろ向き枝刈りの改善を行いました（`dict.rs`の`pattern2`）
- 探索の並列化を行いました
- `--prefix`オプションでパスワードのprefixを指定可能になりました
- `--suffix`オプションでパスワードのsuffixを指定可能になりました
- `--disable-japanese-pruning`オプションで、日本語として不自然なパスワードを除外する枝刈りを無効化できます
- `--verbose`オプションで探索途中の文字列を表示可能になりました
- 辞書の中で`;`を含む行をコメント行として読み飛ばすよう修正しました

### 並列化
探索時のスレッド数は環境変数`RAYON_NUM_THREADS`で指定できます。

### `--disable-japanese-pruning`オプション
日本語として不自然なパスワードを除外する枝刈りでは以下のケースで枝刈りしています
- 子音の後に記号や数値がくる
- 同じ文字が連続する
- 4回以上母音が連続する
- 3回以上子音が連続する

結構大胆な枝刈りをしており、真のパスワードまで枝刈りしてしまう可能性もあるので気になる人は`--disable-japanese-pruning`オプションで無効化するか、`pruning.rs`の`validate_natural_japanese`関数を修正し自分で判定条件をカスタムしてください。

### `--prefix`, `--suffix`オプションについて
衝突するパスワードが多数あり、ランダムに生成した文字列では無意味な文字列がヒットするだけなので、`--prefix`もしくは`--suffix`を指定して意味のあるパスワードを探索することを強く推奨します。
