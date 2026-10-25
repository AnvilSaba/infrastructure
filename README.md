# Infrastructure

あんびる鯖のクラスタに維持する構成を GitOps で管理します。

アプリのソースコード、DB マイグレーション、コンテナのビルド、Helm Chart のソースは [platform](https://github.com/AnvilSaba/platform) で管理します。
このリポジトリではクラスタ構成、環境ごとの設定、暗号化済み Secret、配置する成果物のバージョンを管理します。

## 開始方法

mise と Git を用意し、リポジトリルートで実行します。

```powershell
mise trust
mise install
mise run fmt:check
```

## ドキュメント

- [開発環境と確認手順](docs/development.md)
- [クラスタの構築とデプロイ](docs/deployment.md)
- [Flux の初期構築](docs/flux.md)
- [Secret / Config の準備と運用](docs/secret-config.md)
- [staging の構築と更新 PR](docs/staging.md)
- [production 更新 PR](docs/production-pr.md)
- [既存 DB の CloudNativePG 移行](docs/cloudnativepg-migration.md)
- [設計上の決定](docs/adr/0001-separate-gitops-desired-state.md)

## ライセンス

[MIT License](LICENSE) で公開しています。
