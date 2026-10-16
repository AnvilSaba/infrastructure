# GitOps成果物の識別子と実行順序

## 確認できた事実

- **Git source revision** はGitリポジトリ内のコミットを指す識別子であり、OCIイメージのタグやOCIダイジェストとは別の名前空間・対象である。Gitはコミット間の祖先関係を判定する。 [Git `merge-base`](https://git-scm.com/docs/git-merge-base)
- Dockerは `NAME[:TAG|@DIGEST]` 形式を受け付ける。タグを指定しない場合は `latest` が使われる。タグは最新イメージを得る用途として説明され、同じタグを後日pullすれば更新されたイメージになり得る。従ってタグは可変の参照であり、内容そのものの固定識別子ではない。 [Docker `image pull`](https://docs.docker.com/reference/cli/docker/image/pull/)
- Dockerの例ではイメージのダイジェストは `sha256:…` 形式で表示され、`name@sha256:…` でその版を固定してpullできる。Dockerはdigest指定を同じ版を保証する方法として説明する。OCI manifest digestはコンテンツを識別するダイジェストであり、`sha-<gitsha>` 形式のタグとは意味が異なる。 [Docker `image pull`](https://docs.docker.com/reference/cli/docker/image/pull/)
- `sha-<gitsha>` はタグ文字列にGit commit SHAを埋め込んだ規約として解釈できる。これはタグでGit source revisionを示すものであり、そのタグが参照するOCIコンテンツの `sha256:…` digestそのものではない。**この形式・運用規約の仕様は提示資料には記載されておらず、この行は用語の区別に関する推論。** [Docker `image pull`](https://docs.docker.com/reference/cli/docker/image/pull/)
- `git merge-base --is-ancestor <current> <candidate>` は、最初のコミットが2番目の祖先なら終了status `0`、祖先でなければ `1`。status `1` 以外の非ゼロはエラーを表す。したがって `0/1` を単純な真偽値として扱う前に、他の終了値をエラーとして区別する必要がある。 [Git `merge-base`](https://git-scm.com/docs/git-merge-base)
- GitHub Actions の同一concurrency groupでは同時実行を1つに制限でき、`cancel-in-progress: true` は実行中の同グループのjob/workflowをキャンセルする。デフォルトでは保留中の実行も新しい保留実行に置換される。 [GitHub Actions concurrency](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency)
- concurrencyは実行を直列化する設定だが、dispatch時刻順の実行順を保証しない。公式文書は待機開始時刻に基づくFIFOを記述する一方、実際の開始時刻により順序は保証されないと明記する。 [GitHub Actions concurrency](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency)
- **cancelと既送信dispatch:** 公式concurrency資料はworkflow/jobのキャンセルを説明するが、すでに外部へ送信されたdispatchイベントやその受信側の処理を取り消すかは説明していない。ここは提示資料から確認できない。 [GitHub Actions concurrency](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency)
- Flux `OCIRepository` は `.spec.ref.digest: "sha256:<SHA-value>"` をサポートし、この指定は他のref指定より優先される。 [Flux OCI Repositories](https://fluxcd.io/flux/components/source/ocirepositories/)
- Fluxの資料はOCIRepositoryでHelm chart layerを選択する例を示す。従ってOCI形式のchart取得にも `ref.digest` を指定できる。 [Flux OCI Repositories](https://fluxcd.io/flux/components/source/ocirepositories/)

## 証拠の不足・範囲

- `sha-<gitsha>` タグを生成・移動できる主体、レジストリのタグ上書き設定、タグとGit revisionの実際の対応は提示URLからは確認できない。
- イメージの `sha256` digestがOCI manifest digestかindex digestか、マルチプラットフォーム時の選択動作はDocker CLI資料だけでは特定できない。
- 外部dispatchが送信済みの場合の取消し・受信側処理の停止可否は提示資料に記述がない。
- Fluxのチャートlayer例とdigest ref指定は同じOCIRepositoryの機能説明にあるが、具体的なチャートレジストリ／チャートでの取得結果までは検証していない。
