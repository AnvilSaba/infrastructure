# Agent Development Workflow

## Verification

- Run `mise run fmt` and then `mise run fmt:check` from the repository root before completing or committing changes supported by Oxfmt. Install tools with `mise install`; see `docs/development.md` for setup.
- When changing workflows, run `mise exec -- gh actions-lock` and `mise exec -- gh actions-lock --verify`. Run all `gh` commands outside the sandbox. The formatting CI also checks the lockfile locally.
- When manifests are added or changed, render each affected Kustomize root with `mise exec -- kustomize build <path>`. For Helm values, render the pinned chart version with those values and verify application / Migrator compatibility. Rendering alone does not verify reconciliation or connectivity.
- Fix failures before completing the task. Report checks that cannot run and explain why.

## Sandbox execution

When `.git` is read-only inside the sandbox, run Git writes such as `git add` and `git commit` outside the sandbox from the outset. Read-only Git commands can run inside the sandbox.
