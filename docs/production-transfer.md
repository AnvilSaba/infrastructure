# production 移管先の対応

移管元は platform の `feature/gitops-deployment`、準備時点の commit は `bdfb2ec`。
この変更は移管先の準備だけで、稼働クラスタの source・停止設定・配置版は変更しない。

| 移管元 (platform)                     | 移管先 (infrastructure)                                     |
| ------------------------------------- | ----------------------------------------------------------- |
| deploy/flux/production                | clusters/production                                         |
| deploy/flux/controllers               | infrastructure/base/controllers                             |
| deploy/postgres/operator              | infrastructure/base/operator                                |
| deploy/postgres/cloudnativepg         | infrastructure/base/database                                |
| deploy/postgres/import                | infrastructure/production/database                          |
| deploy/postgres/roles                 | infrastructure/base/roles + infrastructure/production/roles |
| deploy/config/production              | config/production                                           |
| deploy/flux/apps                      | apps/production/platform                                    |
| deploy/helm/platform/values.prod.yaml | apps/production/platform/environment-values.yaml            |

GitRepository 名 `platform`、Flux Kustomization 名、HelmRelease/OCIRepository 名、
namespace `anvilsaba`、releaseName/targetNamespace/storageNamespace を維持する。
GitRepository URL と Kustomization の path だけが source 移管に必要な差分となる。
ConfigMap の名前・ラベル・内容も維持する。SOPS ciphertext と production 受信者は変更しない。
共通の DB/roles 定義に namespace を埋め込まず、production overlay で元の namespace を付ける。

`platform-config.spec.suspend`、`platform-db.spec.suspend`、
`HelmRelease/platform.spec.suspend` は true のまま。
prune/deletionPolicy、dependsOn、healthChecks/healthCheckExprs と timeout を引き継ぐ。
import は既存 Cluster の bootstrap 定義を維持するために残し、再実行を前提としない。

## 確認

各対応先で `mise exec -- kustomize build <path>` を実行し、
元と比較する。クラスタ入口は URL/path 以外の差分を許容しない。
アプリの比較では ConfigMap の集約による順序以外の差分を許容しない。
Chart 1.0.0 と既存 values も描画して比較する。

実際の source 切り替え前に、移管元の最新 main とクラスタの inventory、
resource UID、Helm release、停止設定、DB/PVC を再確認する。
今回の準備時点のコピーを、そのまま最新状態だと扱わない。
source の引き継ぎと digest 対応 Chart/image の導入は別段階とする。
旧定義・旧更新経路は Issue #6 の引き継ぎ確認まで維持する。
production 更新 PR は source 移管が完了するまで merge しない。
