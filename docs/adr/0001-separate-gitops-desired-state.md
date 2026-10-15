---
status: accepted
---

# GitOps の desired state を専用リポジトリで管理する

アプリケーションの開発・成果物の公開と、環境へ配置するバージョンの決定を分けるため、GitOps の desired state を `AnvilSaba/platform` から `AnvilSaba/infrastructure` へ移す。
`platform` はアプリケーションコード、DB マイグレーション、コンテナビルド、Helm Chart ソースを所有し、`infrastructure` はクラスタ構成、環境別設定、SOPS で暗号化した Secret、配置する成果物の参照と更新処理を所有する。
同一リポジトリで管理し続ける場合に比べ、リポジトリ間の通知・認証・変更の追跡が必要になるが、環境への配置判断と履歴を desired state 側に集約できる。

## 環境別の構成

Flux の入口を `clusters/{staging,production}/`、CloudNativePG などの基盤定義を `infrastructure/{base,staging,production}/`、アプリケーション定義を `apps/{base,staging,production}/platform/` に配置する。
環境別の app config と暗号化済み Secret、および暗号化ルールの `.sops.yaml` も `infrastructure` が所有する。
Chart ソースは `platform` に残し、配置対象の Chart と image の参照を `infrastructure` で管理する。

staging と production は別 namespace に配置する。
将来別サーバー・別クラスタへ配置できるよう、staging の構築・稼働は production の resource、DB、Secret、外部サービスの設定や稼働状態に依存させない。
共通定義を利用する場合も、production の環境定義を参照元にせず、クラスタ共通基盤の準備に production の稼働を前提としない。
SOPS の復号鍵も環境別に分け、production は既存鍵を維持し、staging は専用鍵を用意する。

## 成果物の公開と配置の更新

staging は `platform/main` での各 image build 成功後に、`repository_dispatch` で component、image reference、source revision を通知する。
`infrastructure` の workflow が対応する `apps/staging/platform/` の参照を `main` に commit し、Flux が反映する。
`db-migrator` も、その image build が発生した場合は成果物の参照を staging の desired state に反映する。
アプリの更新に必要な Migrator が未公開の場合、そのアプリの desired state 更新を待ち、必要な成果物が揃ってから一緒に更新する。
無関係な component の build 完了は待たない。
Chart も `main` の変更を検証対象とし、変更時に source revision を識別できる検証用成果物を公開して staging の参照を固定する。

production は `platform` の release workflow が成果物の publish 完了後に通知し、`infrastructure` が component と release の整合性を検証して、application image・Helm Chart・db-migrator の該当する参照を更新する PR を作成できるようにする。
公開後に PR を作成するかを切り替え可能にし、成果物の公開と production 更新 PR の作成を分離する。
`platform` の各 release 実行時に PR の自動作成を選択し、既定では作成しない。
また、`infrastructure` の Actions を手動実行し、複数 component の公開済み成果物をまとめた production 更新 PR を作成できるようにする。
手動実行では component ごとに公開済み release tag を明示し、未指定の component は現在の production の参照を維持する。
`promote-production.rs` が担う release tag、Git revision、db-migrator revision の整合性検証も `infrastructure` に移す。
複数アプリをまとめる場合は異なる release revision の組み合わせを許可し、選択したアプリの release revision をすべて履歴に含む公開済み Migrator を使用する。
この関係を検証できない組み合わせは拒否し、DB 変更の後方互換性は引き続き必要とする。
staging での正常稼働との自動連動は必須にせず、成果物の公開と整合性検証後に PR を作成し、人がレビューして昇格を判断する。
production の desired state は PR の merge を経由して更新し、Flux が反映する。

## 成果物の識別と更新順序

staging / production とも、application image・db-migrator・OCI 形式の Helm Chart は digest で配置対象を固定する。
Git の source revision、SHA tag、release tag は追跡情報として残し、ソースの識別と完成した成果物の識別を区別する。
digest 対応に必要な Chart の改修は `platform` で行い、既存環境の source 移管とは分ける。

`main` の同じ component のビルドは共通の concurrency group に入れ、新しい実行で古い実行をキャンセルする。
release 用ビルドはこのキャンセル対象から分離する。
ビルドのキャンセルだけには依存せず、受信側でも component ごとの source revision を Git 履歴で比較し、現在の参照より新しい子孫 revision への自動更新だけを許可する。
同じ revision と digest の通知は変更なしとし、古い revision の通知では巻き戻さず、履歴の前後関係を確認できない場合は更新を止める。
更新直前にも最新の desired state と照合し、通知の到着順やビルド時刻を更新順序の根拠にしない。

意図した production アプリの切り戻しは手動 PR で行い、Migrator は以前の revision へ戻さない。
Migrator の整合性は、更新対象だけでなく据え置くアプリも含めて確認する。
識別子と順序判定に関する根拠は [公式資料の調査メモ](../research/gitops-artifact-identity-and-ordering.md) に記録する。

## リポジトリ間の認証

通知送信と desired state 更新には GitHub App の短期 token を使用し、用途ごとに認証情報を分けて、対象リポジトリと権限を限定する。
`repository_dispatch` の送信にも対象リポジトリの `Contents: write` が必要なため、通知用 token もリポジトリへの書き込み権限を持つものとして管理する。
認証要件の根拠は [GitHub 公式資料の調査メモ](../research/github-gitops-notification-auth.md) に記録する。

## 既存環境の引き継ぎ

両環境の desired state と Flux 定義を準備してから、Flux の `GitRepository` と各 `Kustomization.spec.path` を新しいリポジトリへ切り替える。
切り替えでは既存 resource と新しい定義の対応を確認し、Flux が同一 resource の管理を継続できるようにする。
既存 resource の識別情報と管理関係を維持し、旧定義の削除による prune と、新旧の同期による競合を避ける順序で移行する。
今回の移管には既存の停止設定の解除、配置バージョンの変更、DB の切り替えを含めず、既存環境の状態を引き継ぐ。
移行完了後は `infrastructure/main` を staging / production の desired state の正とする。
