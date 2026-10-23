# GitHub Actions 通知認証に関する公式資料の事実

## 確認できた事実

1. `repository_dispatch` API が受け付ける fine-grained token は、GitHub App user access token、GitHub App installation access token、fine-grained personal access token の3種類。必要な権限は対象リポジトリの `Contents: write`。 ([GitHub REST API: Create a repository dispatch event](https://docs.github.com/en/rest/repos/repos#create-a-repository-dispatch-event))

2. `GITHUB_TOKEN` は Actions を実行しているリポジトリに限定された GitHub App installation access token。このため、別リポジトリへの書き込み権限は持たず、`platform` の token で `infrastructure` に `repository_dispatch` を送信する用途には使えない。 ([GitHub Docs: GITHUB_TOKEN](https://docs.github.com/en/actions/concepts/security/github_token))

3. GitHub App installation access token は発行から1時間で失効する。発行時に `repositories` または `repository_ids` を指定すると個別リポジトリに絞れる。指定しない場合は、アプリのインストール先として許可された全リポジトリにアクセスできる。インストールに許可されていないリポジトリは指定できない。 ([GitHub Docs: Authenticating as a GitHub App installation](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/authenticating-as-a-github-app-installation))

4. `GITHUB_TOKEN` で作成・更新したコミットによる通常のイベントは新しいワークフロー実行を起こさない。`workflow_dispatch` と `repository_dispatch` は例外として常に実行を起こす。`GITHUB_TOKEN` で作成・更新した PR の `opened`、`synchronize`、`reopened` による `pull_request` ワークフローは承認待ちになる。他の PR 活動種別では実行されない。 ([GitHub Docs: Triggering a workflow](https://docs.github.com/en/actions/how-tos/writing-workflows/choosing-when-your-workflow-runs/triggering-a-workflow))

5. GitHub App installation token を使ってワークフロー内からイベントを起こした場合、トークンが必要なイベントで後続ワークフローを起動でき、作成・更新された PR の `pull_request` ワークフローも承認なしで自動実行できる。 ([GitHub Docs: Triggering a workflow](https://docs.github.com/en/actions/how-tos/writing-workflows/choosing-when-your-workflow-runs/triggering-a-workflow))

## 明示すべき注意点

`repository_dispatch` API の fine-grained token 要件は `Contents: write` である。そのため、このトークンを「dispatch 専用」とは主張できない。 ([GitHub REST API: Create a repository dispatch event](https://docs.github.com/en/rest/repos/repos#create-a-repository-dispatch-event))

## 情報の欠落・不確実性

- 提供された公式資料の記載に、上記の範囲で未解決の矛盾は見つからなかった。
- 「別リポジトリへの書き込み権限を持たない」は、`GITHUB_TOKEN` の権限がワークフローを含むリポジトリに限定されるという公式記述からの帰結。
- この調査では提示URL以外を探索していない。ここに記した仕様の有効性・個別環境での権限設定や実行結果は別途確認していない。
