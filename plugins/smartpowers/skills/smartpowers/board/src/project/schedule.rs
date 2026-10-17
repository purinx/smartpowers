//! The execution order of a project's tasks, read from `tasks/PLAN.md`.

/// Task numbers in the order `PLAN.md` first mentions their files
/// (`NN_name.md`), which is the plan's execution order.
pub fn plan_order(plan: &str) -> Vec<String> {
    let bytes = plan.as_bytes();
    let mut order: Vec<String> = Vec::new();
    for start in 0..bytes.len() {
        let rest = &bytes[start..];
        let boundary =
            start == 0 || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
        if !boundary
            || rest.len() < 7
            || !rest[..2].iter().all(u8::is_ascii_digit)
            || rest[2] != b'_'
        {
            continue;
        }
        let name_len = rest[3..]
            .iter()
            .take_while(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || **b == b'_')
            .count();
        if name_len > 0 && rest[3 + name_len..].starts_with(b".md") {
            let number = String::from_utf8_lossy(&rest[..2]).into_owned();
            if !order.contains(&number) {
                order.push(number);
            }
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_execution_order_from_the_plan() {
        let plan = "| 02 | [B](02_domain.md) | 01 |\n| 01 | [A](01_infra.md) | None |\nsee 02_domain.md\nx01_no.md";
        assert_eq!(plan_order(plan), ["02", "01"]);
    }
}
