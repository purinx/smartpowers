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
- dependencies and parallel group;
- verification and acceptance criteria;
- the repository-relative task-file path as the durable source reference.
- the Smartpowers project ID so similarly named initiatives remain distinguishable.

Prefix or label remote work with the project ID when the task manager supports it. Represent dependencies with the task manager's native relationship when supported. Otherwise include them explicitly in the description. After creation, add the remote issue identifier and URL to the corresponding local task file under `Remote task:`.

If a remote operation partially fails, report exactly which issues were created, do not retry ambiguous writes blindly, and reconcile by searching before continuing.

If no suitable task-management tool is connected, keep the approved local plan as the source of truth and state that remote synchronization was skipped. Do not block implementation unless the user made synchronization mandatory.
