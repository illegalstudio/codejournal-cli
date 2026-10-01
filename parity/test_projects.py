"""Project metadata and cross-project workflow within an isolated tenant."""
import subprocess
from support import ParityCase


class ProjectParityTest(ParityCase):
    def second_project(self):
        folder = self.base / "second"
        subprocess.run(["git", "clone", "--no-hardlinks", str(self.repo), str(folder)],
                       check=True, capture_output=True)
        subprocess.run(["git", "remote", "set-url", "origin", "https://example.test/parity/second.git"],
                       cwd=folder, check=True, capture_output=True)
        self.invoke("python", "rules", "set", "Synthetic second project.", project=None, cwd=folder)
        self.invoke("rust", "project", "init", project=None, cwd=folder)
        return folder

    def test_project_metadata_paths_and_merge(self):
        folder = self.second_project()
        for engine in self.envs:
            projects = self.invoke(engine, "projects")["projects"]
            self.assertEqual(sorted(item["slug"] for item in projects), ["parity-repo", "parity-second"])
            self.invoke(engine, "project", "edit", "--name", "Renamed repo", "--provides", "cache,journal")
            project = self.invoke(engine, "project", "show")["project"]
            self.assertEqual(project["name"], "Renamed repo")
            self.assertEqual(project["provides"]["manual"], ["cache", "journal"])
            extra = self.base / (engine + "-checkout")
            subprocess.run(["git", "clone", "--no-hardlinks", str(self.repo), str(extra)], check=True, capture_output=True)
            subprocess.run(["git", "remote", "set-url", "origin", "https://example.test/parity/repo.git"],
                           cwd=extra, check=True, capture_output=True)
            self.invoke(engine, "project", "path-add", extra)
            self.invoke(engine, "project", "path-remove", extra)
            self.invoke(engine, "project", "merge", "parity-second", "--into", "parity-repo")
            self.assertEqual([item["slug"] for item in self.invoke(engine, "projects")["projects"]], ["parity-repo"])
            self.invoke(engine, "normalize", "--dry-run")
        self.assertTrue(folder.is_dir())

    def test_forwarded_task_returns_its_resolution(self):
        self.second_project()
        entries = self.add_entry()
        for engine, result in entries.items():
            forwarded = self.invoke(engine, "task", "add", "--to", "parity-second", "--from-entry", result["entry"]["id"])
            task = forwarded["task"]
            self.assertEqual(task["title"], "Cache invalidation")
            identity = task["id"]
            listed = self.invoke(engine, "task", "list", "--forwarded", "--from", "parity-repo", project="parity-second")
            self.assertEqual([item["id"] for item in listed["tasks"]], [identity])
            self.invoke(engine, "task", "done", identity, "--note", "Fixed downstream", project="parity-second")
            brief = self.invoke(engine, "brief")
            self.assertEqual([item["title"] for item in brief["forwarded_results"]], ["Cache invalidation"])
            self.assertEqual(brief["forwarded_results"][0]["resolution"], "Fixed downstream")
