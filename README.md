14文字のパスワードを全列挙するためのブランチです。  
AWS m6g.metalで列挙しました。  
src/config.rsで列挙対象のパスワードを変更できます。

テストはreleaseビルドでないと通りません（テスト中にオーバーフローが発生するため）
