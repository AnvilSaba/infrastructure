# 開発環境と確認手順

## ツールの準備

Git と mise を用意し、リポジトリルートで実行します。

```powershell
mise trust
mise install
mise tasks ls
```

GitHub CLI、Node.js、Helm、kubectl、SOPS、age、Kustomize、Oxfmt のバージョンを `mise.toml` で固定しています。
Node.js は Oxfmt の実行用です。
新しいリリースの導入には7日間の待機期間を設けています。

ツールは `mise run <タスク名>` または `mise exec -- <コマンド>` で実行します。
Git は直接実行できます。エージェントが `gh` を実行する場合は、サンドボックス外で実行します。

## VS Code

推奨拡張機能の Oxc をインストールしてください。
`.vscode/settings.json` で保存時の整形と2スペースのインデントを設定しています。

## 整形

```powershell
mise run fmt
mise run fmt:check
```

変更の完了前・コミット前に両方を実行します。整形 CI も `mise run fmt:check` を実行します。

GitHub Actions の依存関係は `gh-actions-lock` で固定します。
ワークフローの依存関係を変更した場合は、次を実行し、生成されたロックファイルも含めてレビューします。

```powershell
mise exec -- gh extension install github/gh-actions-lock --pin v0.1.6
mise exec -- gh actions-lock
mise exec -- gh actions-lock --verify
```

手順の詳細は [gh-actions-lock の公式 README](https://github.com/github/gh-actions-lock#usage) を参照してください。

## マニフェストの変更

変更した各 Kustomize ルートで `mise exec -- kustomize build <パス>` を実行します。
Helm values を変更した場合は、固定した Chart のバージョンと対象の values を使って描画し、アプリと Migrator の互換性も確認します。
描画だけではクラスタへの反映や接続性は確認できないため、実施した検証と未確認の項目を区別して記録します。

## コミットと PR

Conventional Commits を使い、要約は日本語にします。
例: `chore(gitops): 本番構成を更新する`。

課題の仕様と受け入れ条件は GitHub Issues に記録します。
依存する複数の課題を実装する場合は `AGENTS.md` の stacked PR の規約に従います。
