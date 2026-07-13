import os
import pathlib
import re
import subprocess
import sys
import tempfile
import tomllib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
SETUP = ROOT / "scripts" / "codex-cloud-setup.sh"
MAINTENANCE = ROOT / "scripts" / "codex-cloud-maintenance.sh"
WORKFLOW = ROOT / ".github" / "workflows" / "build.yaml"
DOCUMENTATION = ROOT / "docs" / "CodexCloudEnvironment.md"
REQUIREMENTS = ROOT / "scripts" / "check_conf-requirements.txt"


def read(path):
    return path.read_text(encoding="utf-8")


def write_executable(path, content):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    path.chmod(0o755)


def script_tool_versions(path):
    return dict(
        re.findall(
            r"^install_cargo_tool (cargo-nextest|cargo-tarpaulin|cargo-machete|wasm-pack) ([0-9.]+)$",
            read(path),
            flags=re.MULTILINE,
        ),
    )


def workflow_tool_installs(path):
    aliases = {"nextest": "cargo-nextest"}
    installs = re.findall(
        r"^\s*uses: taiki-e/install-action@([^\s#]+)\s*$\n"
        r"^\s*with:\s*$\n"
        r"^\s*tool: (nextest|cargo-tarpaulin|cargo-machete|wasm-pack)@([0-9.]+)\s*$",
        read(path),
        flags=re.MULTILINE,
    )
    return [(aliases.get(tool, tool), version, revision) for revision, tool, version in installs]


class CodexCloudScriptTests(unittest.TestCase):
    def test_tool_versions_match_github_actions(self):
        installs = workflow_tool_installs(WORKFLOW)
        expected = {tool: version for tool, version, _ in installs}

        self.assertEqual(len(installs), 4)
        self.assertTrue(all(re.fullmatch(r"[0-9a-f]{40}", revision) for _, _, revision in installs))
        self.assertEqual(script_tool_versions(SETUP), expected)
        self.assertEqual(script_tool_versions(MAINTENANCE), expected)

    def test_rust_version_matches_repository_pins(self):
        toolchain_path = ROOT / "rust-toolchain.toml"
        with toolchain_path.open("rb") as file:
            toolchain_version = tomllib.load(file)["toolchain"]["channel"]
        with (ROOT / "Cargo.toml").open("rb") as file:
            manifest_version = tomllib.load(file)["package"]["rust-version"]

        for script in (SETUP, MAINTENANCE):
            match = re.search(r"^required_rust_version=([0-9.]+)$", read(script), re.MULTILINE)
            self.assertIsNotNone(match)
            self.assertEqual(match.group(1), toolchain_version)
        self.assertEqual(manifest_version, toolchain_version)

    def test_scripts_are_executable(self):
        self.assertTrue(os.access(SETUP, os.X_OK))
        self.assertTrue(os.access(MAINTENANCE, os.X_OK))

    def test_script_revisions_match_and_are_documented(self):
        revisions = []
        for script in (SETUP, MAINTENANCE):
            match = re.search(r"^script_revision=([0-9.-]+)$", read(script), re.MULTILINE)
            self.assertIsNotNone(match)
            revisions.append(match.group(1))

        self.assertEqual(revisions[0], revisions[1])
        self.assertIn("Each run begins by printing its script revision", read(DOCUMENTATION))

    def test_maintenance_does_not_execute_repository_checks(self):
        executable_lines = "\n".join(
            line for line in read(MAINTENANCE).splitlines() if not line.lstrip().startswith("#")
        )
        prohibited = (
            r"^\s*(?:cargo\s+(?:bench|build|clippy|fmt|run|test)|"
            r"wasm-pack\s+build|python3?\s+scripts/)"
        )
        self.assertIsNone(re.search(prohibited, executable_lines, re.MULTILINE))

    def test_maintenance_uses_isolated_interpreters_and_trusted_inputs(self):
        maintenance = read(MAINTENANCE)
        self.assertIn('cd /', maintenance)
        self.assertNotIn('cd "$repo"', maintenance)
        self.assertIn('"$python_executable" -I -', maintenance)
        self.assertNotIn('-m pip', maintenance)
        self.assertIn('cmp -s -- "$repo/scripts/check_conf-requirements.txt"', maintenance)
        self.assertIn('unset CARGO_HOME RUSTUP_TOOLCHAIN', maintenance)
        self.assertIn('cargo_command=(rustup run "$rust_channel" env "CARGO_HOME=$cargo_home" cargo)', maintenance)
        self.assertIn('--component clippy,rustfmt,rust-src --no-self-update', maintenance)
        self.assertIn('cargo_home=${CARGO_HOME:-$state_dir/cargo-home}', maintenance)
        self.assertIn('--root "$tool_root"', maintenance)
        self.assertIn('--manifest-path "$repo/Cargo.toml"', maintenance)
        self.assertIn('CARGO_NET_GIT_FETCH_WITH_CLI=false', maintenance)
        self.assertIn('[[ -e "$repo/rust-toolchain" || -L "$repo/rust-toolchain" ]]', maintenance)

    def test_python_isolated_mode_ignores_task_modules_and_pythonpath(self):
        with tempfile.TemporaryDirectory() as directory:
            task_root = pathlib.Path(directory)
            marker = task_root / "executed"
            malicious_module = f"open({str(marker)!r}, 'w').close()\n"
            for module in ("pathlib.py", "tomllib.py", "pip.py"):
                (task_root / module).write_text(malicious_module, encoding="utf-8")
            pip_package = task_root / "pip"
            pip_package.mkdir()
            (pip_package / "__main__.py").write_text(malicious_module, encoding="utf-8")

            environment = os.environ.copy()
            environment["PYTHONPATH"] = str(task_root)
            subprocess.run(
                [sys.executable, "-I", "-c", "import pathlib, tomllib"],
                cwd=task_root,
                env=environment,
                check=True,
            )
            subprocess.run(
                [sys.executable, "-I", "-m", "pip", "--version"],
                cwd=task_root,
                env=environment,
                check=True,
                stdout=subprocess.DEVNULL,
            )
            self.assertFalse(marker.exists())

    def test_requirements_use_only_reviewed_simple_specifiers(self):
        safe_requirement = re.compile(
            r"^[A-Za-z0-9][A-Za-z0-9._-]*"
            r"(?:[<>=!~]=?[A-Za-z0-9.*+_-]+"
            r"(?:,[<>=!~]=?[A-Za-z0-9.*+_-]+)*)?$"
        )
        for line in read(REQUIREMENTS).splitlines():
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            self.assertRegex(stripped, safe_requirement)

    def test_documented_validation_includes_ci_lint_and_runtime(self):
        documentation = read(DOCUMENTATION)
        self.assertIn("cargo machete", documentation)
        self.assertIn("CARGO_HOME=/root/.cache/ftml-codex-cloud/cargo-home", documentation)
        self.assertIn("Python package version | 3.13", documentation)
        self.assertIn("RUSTFLAGS=-A unused -D warnings", documentation)

    def test_setup_excludes_wikijump_service_stack(self):
        setup = read(SETUP).lower()
        for excluded in (
            "docker",
            "libmagic",
            "minio",
            "nodejs",
            "postgresql",
            "redis-server",
            "sqlx-cli",
        ):
            self.assertNotIn(excluded, setup)

    def test_mocked_setup_and_maintenance_are_repeatable_and_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            repo = root / "repo"
            home = root / "home"
            state = root / "state"
            mock_bin = root / "mock-bin"
            tool_bin = root / "tool-bin"
            tool_root_bin = state / "cargo-tools" / "bin"
            marker = root / "task-python-imported"
            rustup_log = root / "rustup.log"
            repo.mkdir()
            home.mkdir()
            mock_bin.mkdir()
            tool_bin.mkdir()
            tool_root_bin.mkdir(parents=True)
            (repo / "scripts").mkdir()
            (repo / ".cargo").mkdir()
            (repo / "Cargo.toml").write_text('[package]\nname = "ftml"\nversion = "1.0.0"\nedition = "2024"\nrust-version = "1.95.0"\n', encoding="utf-8")
            (repo / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "1.95.0"\n', encoding="utf-8")
            (repo / "scripts" / "check_conf-requirements.txt").write_text("inflection>=0.5.0\n", encoding="utf-8")
            (repo / ".cargo" / "config.toml").write_text('[registry]\nglobal-credential-providers = ["cargo:token-from-stdout task-helper"]\n', encoding="utf-8")
            malicious_module = f"open({str(marker)!r}, 'w').close()\n"
            for module in ("pathlib.py", "tomllib.py", "pip.py"):
                (repo / module).write_text(malicious_module, encoding="utf-8")
            subprocess.run(["git", "init", "--quiet", str(repo)], check=True)

            write_executable(
                mock_bin / "python3",
                f'''#!/usr/bin/env bash
set -eu
if [[ ${{1-}} == -I && ${{2-}} == -c ]]; then
  printf '3.13\\n'
elif [[ ${{1-}} == -I && ${{2-}} == -m && ${{3-}} == pip ]]; then
  exit 0
else
  exec {sys.executable} "$@"
fi
''',
            )
            write_executable(
                mock_bin / "sudo",
                '''#!/usr/bin/env bash
set -eu
if [[ ${1-} == install ]]; then
  shift
  exec /usr/bin/install "$@"
fi
exit 0
''',
            )
            write_executable(
                home / ".cargo" / "bin" / "rustup",
                '''#!/usr/bin/env bash
set -eu
if [[ -n ${RUSTUP_TOOLCHAIN-} ]]; then
  printf 'RUSTUP_TOOLCHAIN leaked into rustup: %s\\n' "$RUSTUP_TOOLCHAIN" >&2
  exit 70
fi
if [[ -n ${CARGO_HOME-} ]]; then
  printf 'CARGO_HOME leaked into rustup: %s\\n' "$CARGO_HOME" >&2
  exit 74
fi
printf '%s|%s\\n' "$PWD" "$*" >>"$MOCK_RUSTUP_LOG"
if [[ ${1-} == toolchain ]]; then
  [[ $* == *--no-self-update* ]] || exit 75
  exit 0
fi
if [[ ${1-} == target ]]; then
  exit 0
fi
if [[ ${1-} != run || ${2-} != 1.95.0 ]]; then
  exit 71
fi
shift 2
if [[ ${1-} == env ]]; then
  [[ ${2-} == "CARGO_HOME=$MOCK_CARGO_HOME" ]] || exit 76
  shift 2
fi
if [[ ${1-} == rustc && ${2-} == --version ]]; then
  printf 'rustc 1.95.0 (mock)\\n'
elif [[ ${1-} == cargo && ${2-} == --version ]]; then
  printf 'cargo 1.95.0 (mock)\\n'
elif [[ ${1-} == cargo && ${2-} == install && ${3-} == --list ]]; then
  printf 'cargo-nextest v0.9.100:\\n    cargo-nextest\\ncargo-tarpaulin v0.32.8:\\n    cargo-tarpaulin\\ncargo-machete v0.8.0:\\n    cargo-machete\\nwasm-pack v0.13.1:\\n    wasm-pack\\n'
elif [[ ${1-} == cargo && ${2-} == fetch ]]; then
  [[ $PWD == / ]]
  [[ $* == *'--manifest-path '*'/Cargo.toml'* ]]
else
  printf 'Unexpected rustup command: %s\\n' "$*" >&2
  exit 72
fi
''',
            )

            versions = {"cargo-nextest": "0.9.100", "cargo-tarpaulin": "0.32.8", "cargo-machete": "0.8.0", "wasm-pack": "0.13.1"}
            for crate, version in versions.items():
                build_behavior = '[[ ${CARGO_HOME-} == "$MOCK_CARGO_HOME" ]]\n  mkdir -p "$2/pkg"' if crate == "wasm-pack" else ":"
                write_executable(
                    tool_root_bin / crate,
                    f'''#!/usr/bin/env bash
set -eu
if [[ ${{1-}} == --version ]]; then
  printf '{crate} {version}\\n'
elif [[ {crate!r} == wasm-pack && ${{1-}} == build ]]; then
  {build_behavior}
else
  exit 73
fi
''',
                )

            environment = os.environ.copy()
            environment.update(
                {
                    "HOME": str(home),
                    "PATH": f"{mock_bin}:/usr/bin:/bin",
                    "PYTHONPATH": str(repo),
                    "RUSTUP_TOOLCHAIN": "task-override",
                    "CARGO_HOME": str(state / "cargo-home"),
                    "FTML_CODEX_REPO": str(repo),
                    "FTML_CODEX_STATE_DIR": str(state),
                    "FTML_CODEX_BIN_DIR": str(tool_bin),
                    "MOCK_RUSTUP_LOG": str(rustup_log),
                    "MOCK_CARGO_HOME": str(state / "cargo-home"),
                }
            )

            for script in (SETUP, SETUP, MAINTENANCE, MAINTENANCE):
                result = subprocess.run([str(script)], cwd=repo, env=environment, text=True, capture_output=True)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

            self.assertFalse(marker.exists())
            self.assertFalse((repo / "pkg").exists())
            fetches = [line for line in read(rustup_log).splitlines() if "cargo fetch" in line]
            self.assertEqual(len(fetches), 4)
            self.assertTrue(all(line.startswith("/|") for line in fetches))
            for crate, version in versions.items():
                output = subprocess.run([str(tool_bin / crate), "--version"], text=True, capture_output=True, check=True).stdout
                self.assertIn(version, output)

            requirements = repo / "scripts" / "check_conf-requirements.txt"
            requirements.write_text("inflection>=0.6.0\n", encoding="utf-8")
            changed = subprocess.run([str(MAINTENANCE)], cwd=repo, env=environment, text=True, capture_output=True)
            self.assertNotEqual(changed.returncode, 0)
            self.assertIn("Python requirements changed", changed.stderr)

            requirements.write_text("inflection>=0.5.0\n", encoding="utf-8")
            (repo / "rust-toolchain").write_text("nightly\n", encoding="utf-8")
            overridden = subprocess.run([str(MAINTENANCE)], cwd=repo, env=environment, text=True, capture_output=True)
            self.assertNotEqual(overridden.returncode, 0)
            self.assertIn("extensionless rust-toolchain", overridden.stderr)


if __name__ == "__main__":
    unittest.main()
