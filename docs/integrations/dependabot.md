# Dependabot

## Scope

`.github/dependabot.yml` configures weekly version checks for:

- npm at `/`;
- Cargo at `/src-tauri`.

Each ecosystem is limited to three open version-update pull requests. Safe
minor and patch updates are grouped; automatic major version updates are
ignored. Dependabot does not merge pull requests, and this repository adds no
auto-merge workflow.

The configured `dependencies`, `frontend`, and `rust` labels must exist in the
GitHub repository for all requested labels to be applied. Missing labels do
not justify changing dependency behavior; create them in repository settings
or with the GitHub CLI before relying on the labels.

## Activation

GitHub reads this file from the repository default branch. Merging the commit
into `main` enables version-update scheduling; its presence on a feature branch
does not prove that Dependabot is active.

In **Settings → Security → Advanced Security**, verify separately:

1. Dependency graph is enabled.
2. Dependabot alerts are enabled.
3. Dependabot security updates are enabled if desired.

Those security settings are not controlled by `dependabot.yml`. No GitHub
setting was changed during local implementation.

## Validation

Local validation checks:

- YAML parses successfully;
- `version` is `2`;
- exactly npm `/` and Cargo `/src-tauri` are configured;
- both schedules are weekly;
- both limits equal 3;
- each ecosystem groups minor/patch and ignores SemVer majors;
- `package-lock.json` and `src-tauri/Cargo.lock` exist.

GitHub-side acceptance requires the file on the default branch and a successful
Dependabot run visible under **Insights → Dependency graph → Dependabot**.

## Cost, dependencies, and rollback

No runtime dependency, secret, paid service, or auto-merge behavior is added.
Observed cost is zero. Rollback is deleting `.github/dependabot.yml`; that
stops version-update scheduling but does not disable security alerts configured
in repository settings.
