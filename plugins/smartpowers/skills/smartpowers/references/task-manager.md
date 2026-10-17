# Task-manager integration guide

Use this guide after the detailed local plan is approved and a connected task-management tool such as Linear is available.

## Before remote writes

1. Discover accessible teams and projects with read-only calls.
2. Ask the user to identify the exact target project if it is not already unambiguous.
3. Present the proposed mapping from local task files to remote issues, including parent/child relationships, execution order, and labels if applicable.
4. Obtain explicit confirmation of the target project and mapping.
5. Search the target project for existing issues with the same task-file path or title to avoid duplicates.

Do not create a new project, team, cycle, label, or milestone unless the user specifically asks for it.

## Issue mapping

Create one remote issue per numbered task file. Preserve the local task title and include:

- objective and scope;
- covered `REQ-*` and `NFR-*` IDs;
- dependencies;
- verification and acceptance criteria;
- the repository-relative task-file path as the durable source reference.
- the Smartpowers project ID so similarly named initiatives remain distinguishable.

Prefix or label remote work with the project ID when the task manager supports it. Represent dependencies with the task manager's native relationship when supported. Otherwise include them explicitly in the description. After creation, add the remote issue identifier and URL to the corresponding local task file under `Remote task:`.

## Progress sync

When local tasks are linked to Linear (or another connected task manager) through `Remote task:`, the remote issue must always reflect current progress during implementation. Update it at the moment each event happens, not at the end of the session:

- **Start:** move the issue to the team's in-progress state (e.g. `In Progress`) and assign it to the current user when unassigned, right before starting work on the task.
- **Progress:** add a short comment after each meaningful step, such as a completed sub-step, a passing verification command, or a commit, including what changed and what remains.
- **Blocked:** set the blocked state or label if one exists, and comment with the blocker and what is needed to resolve it.
- **Scope change:** comment when implementation returns the project to an earlier approval gate, naming the affected requirements.
- **Done:** after the task's verification passes, comment with a summary, commits or PR links, and verification results, then move the issue to the team's completed state.

Use the team's existing workflow states; discover them with read-only calls instead of guessing names, and do not create new states. Keep the local task file's `Status:` and the remote state consistent. If a remote update fails, continue local work only after telling the user which update is missing, then retry or reconcile before the next status change.

If a remote operation partially fails, report exactly which issues were created, do not retry ambiguous writes blindly, and reconcile by searching before continuing.

If no suitable task-management tool is connected, keep the approved local plan as the source of truth and state that remote synchronization was skipped. Do not block implementation unless the user made synchronization mandatory.
