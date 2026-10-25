# production 更新 PR

production の更新候補は `automation/production` ブランチの PR にまとまる。

## Git 更新用 App

Git の push と PR 作成には [共通の Git 更新用 App](flux.md#git-更新用-app)を使用する。
PR は GitHub Actions の **Artifact update PR** が作成する。

## 初回有効化

1. [Flux の Git 更新用 App](flux.md#git-更新用-app)を設定する。
2. digest に対応した安定版 Chart と各 image の公開を確認する。
3. GitRepository `platform-image-updates` と production の各 ImagePolicy が Ready になったら、`infrastructure/production/image-automation/production.yaml` の ImageUpdateAutomation `platform-production` の `spec.suspend` を Git で `false` にする。
4. 初回 PR で Chart と全 image の digest が埋まり、意図した版を示していることを確認する。HelmRelease の停止解除は [本番デプロイ手順](deployment.md#initial-setup)に従う。

## レビューと反映

アプリと Chart は安定版 SemVer、Migrator は `release` の更新が候補になる。各成果物の公開タイミングは異なるため、PR 作成時点で必要な成果物がすべて揃っているとは限らない。

1. CI と差分、Chart と image の tag / digest を確認する。
2. staging の実行結果、アプリと Migrator の組み合わせ、SQL の後方互換性を確認する。未確認の項目は staging で検証する。
3. PR を squash merge し、更新ブランチを削除する。次の候補を最新 main から作成できるようにする。
4. [本番デプロイ後の確認](deployment.md#3-デプロイ後の確認)を行う。

## 手動指定と切り戻し

特定版や複数 component をまとめて指定するときは、main から別ブランチを作り、
`apps/production/platform/values.yaml` / `chart.yaml` の tag と対応する公開済み digest を PR で変更する。
変更しない component の参照は維持する。

自動更新で上書きされないよう、`infrastructure/production/image-automation/production.yaml` の対象 ImagePolicy も同じ PR で固定する。
アプリ / Chart は SemVer range を特定版へ絞る。
Migrator を固定するときは filterTags を既存の完全 SHA タグへ変更して digestReflectionPolicy を IfNotPresent にし、interval を削除する。
自動追従を再開するときは policy の範囲を戻す。

アプリの切り戻しでは Migrator の digest を維持する。DB のロールバックは行わず、旧アプリとの後方互換性を確認する。

## 更新 PR の問題への対処

main と競合する場合は更新候補が最新 main から再生成されるのを待ち、最新の CI と差分を再確認する。
`automation/production` の成果物参照は手作業で編集しない。
PR の自動作成に失敗した場合は、ブランチの差分を確認し、GitHub Actions の **Artifact update PR** を workflow_dispatch から再実行する。`environment` に `production` を選ぶ。
