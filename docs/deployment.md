# 本番デプロイ手順

production のクラスタ構築とデプロイの手順です。コマンド例は Bash 用です。k3s の導入はサーバーで、それ以降のクラスタ操作は mise と kubeconfig を設定した管理端末で実行します。
従来の StatefulSet を使用している環境は、Chart を更新する前に [既存 DB の移行手順](cloudnativepg-migration.md) を実施してください。

## 1. デプロイ環境のセットアップ

### 1.1 Linux サーバー

Ubuntu または Debian などの Linux サーバーを用意します。

必要なものは次のとおりです。

- k3s
- kubectl
- Helm
- Public な GHCR イメージ
- Cloudflare remotely-managed Tunnel とトークン
- Bot の本番設定
- アプリ・Migrator ごとの DB ロールのパスワードを持つ Secret

### 1.2 k3s

`infrastructure/base/k3s/config.yaml` をサーバー上の次の場所へ配置してから k3s を起動します。

```text
/etc/rancher/k3s/config.yaml
```

```bash
sudo mkdir -p /etc/rancher/k3s
sudo cp infrastructure/base/k3s/config.yaml /etc/rancher/k3s/config.yaml
curl -sfL https://get.k3s.io | sh -
sudo systemctl enable --now k3s
```

接続を確認します。

```bash
sudo k3s kubectl get nodes
sudo k3s kubectl get storageclass
```

以降は `sudo` なしで実行するため、kubeconfig を設定します。

```bash
mkdir -p ~/.kube
sudo cp /etc/rancher/k3s/k3s.yaml ~/.kube/config
sudo chown "$USER":"$USER" ~/.kube/config
chmod 600 ~/.kube/config
echo 'export KUBECONFIG="$HOME/.kube/config"' >> ~/.zshrc
source ~/.zshrc
mise exec -- kubectl get nodes
```

### 1.3 Secret と設定

[Secret / Config の準備と運用](secret-config.md) に従い、登録済みの production 設定と復号鍵を確認します。

<a id="initial-setup"></a>

### 初回の DB 準備と配置順序

1. [Flux の初回接続](flux.md#初回の接続)を実施します。Config・DB・HelmRelease の停止設定は維持します。
2. 空の DB を作成する場合は、適用前に `infrastructure/production/database/kustomization.yaml` の import patch を外します。既存 DB がある場合は [DB 移行手順](cloudnativepg-migration.md)に従ってください。
3. `clusters/production/config.yaml` の `spec.suspend` を Git で `false` にし、`platform-config` の Ready を確認します。
4. `clusters/production/database.yaml` の `platform-db.spec.suspend` を Git で `false` にし、`platform-db` と `platform-db-roles` の Ready を確認します。
5. [production 更新 PR の初回有効化](production-pr.md#初回有効化)に従い、Chart と全 image の参照を準備します。`apps/production/platform/release.yaml` の `spec.suspend` を Git で `false` にし、HelmRelease の Ready と Migrator の正常完了を確認します。

### 1.4 Cloudflare Tunnel

Cloudflare 側の Published application の Service URL を次に設定します。

```text
http://public-api:8080
```

## 2. production 更新 PR

[production 更新 PR](production-pr.md)に従い、更新候補をレビューして merge します。反映後は次の確認を行います。

## 3. デプロイ後の確認

稼働中の Pod のイメージを確認します。

```bash
mise exec -- kubectl get pods -n anvilsaba \
  -o custom-columns='POD:.metadata.name,IMAGE:.status.containerStatuses[*].image,IMAGE_ID:.status.containerStatuses[*].imageID'
```

```bash
mise exec -- helm status platform -n anvilsaba
mise exec -- kubectl get pods,deploy,statefulset,service,pvc -n anvilsaba
mise exec -- kubectl get events -n anvilsaba --sort-by=.lastTimestamp
```

各ワークロードが Ready になることを確認します。

```bash
mise exec -- kubectl rollout status deployment/bot -n anvilsaba --timeout=5m
mise exec -- kubectl rollout status deployment/mc-link-server -n anvilsaba --timeout=5m
mise exec -- kubectl rollout status deployment/public-api -n anvilsaba --timeout=5m
mise exec -- kubectl rollout status deployment/cloudflared -n anvilsaba --timeout=5m
mise exec -- kubectl wait cluster/platform-db -n anvilsaba --for=condition=Ready --timeout=5m
mise exec -- kubectl get databaserole -n anvilsaba
```

ログを確認します。

```bash
mise exec -- kubectl logs -n anvilsaba deployment/bot --tail=100
mise exec -- kubectl logs -n anvilsaba deployment/mc-link-server --tail=100
mise exec -- kubectl logs -n anvilsaba deployment/public-api --tail=100
mise exec -- kubectl logs -n anvilsaba deployment/cloudflared --tail=100
mise exec -- kubectl logs -n anvilsaba -l cnpg.io/cluster=platform-db --tail=100
```

Bot の Discord アプリケーションコマンドは、実行中の Pod に TTY 付きで接続して操作します。

```bash
mise exec -- kubectl attach -n anvilsaba -it --detach-keys=ctrl-c deployment/bot
```

Discord 接続の READY 後、コンソールがコマンドを受け付けるようになります。
利用可能なコマンドは `help` で確認可能です。
上記の接続では Ctrl+C で接続だけが終了し、Bot は稼働を続けます。`--detach-keys` を省略した場合の切断キーは Ctrl+P、続けて Ctrl+Q です。

次を確認して完了とします。

- Bot・MC Link Server・公開 API が正常起動し、Bot が Discord へ接続できる
- Cloudflare 経由で `/whitelist.json` を取得できる
- `mc-link-server` Service が本番の `25600/TCP` をコンテナ内の `25565/TCP` へ転送している
- PostgreSQL 再起動後もデータが残る
- Secret の実値が Git やログに含まれていない
