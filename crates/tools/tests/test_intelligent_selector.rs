use std::collections::HashMap;
use tools::{ToolSpec, UsageGuide};
use tools::intelligent_selector::{IntelligentToolSelector, SelectorConfig, ToolSelectionContext, TaskComplexity, UrgencyLevel, UserExpertise};

fn mk_guide(tags: Vec<&str>, caps: Vec<&str>, good_for: Vec<&str>, latency: &str, risk: u8) -> UsageGuide {
    UsageGuide {
        usage_title: "t".into(),
        usage_summary: "s".into(),
        preconditions: vec![],
        arguments_brief: HashMap::new(),
        good_for: good_for.into_iter().map(|s| s.to_string()).collect(),
        not_for: vec![],
        constraints: vec![],
        examples: vec![],
        platforms: vec![],
        cost_class: "free".into(),
        latency_class: latency.into(),
        side_effects: vec![],
        risk_score: risk,
        capabilities: caps.into_iter().map(|s| s.to_string()).collect(),
        tags: tags.into_iter().map(|s| s.to_string()).collect(),
    }
}

fn mk_spec(name: &str, desc: &str, guide: Option<UsageGuide>) -> ToolSpec {
    ToolSpec {
        name: name.into(),
        description: desc.into(),
        usage: format!("{} <args>", name),
        examples: vec![format!("{} example", name)],
        input_schema: "{}".into(),
        usage_guide: guide,
        permissions: None,
        supports_dry_run: false,
    }
}

fn ctx(query: &str, urgency: UrgencyLevel) -> ToolSelectionContext {
    ToolSelectionContext {
        user_query: query.into(),
        session_context: HashMap::new(),
        previous_tools_used: vec![],
        task_complexity: TaskComplexity::Simple,
        urgency_level: urgency,
        user_expertise: UserExpertise::Advanced,
    }
}

fn low_threshold_config() -> SelectorConfig {
    let mut cfg = SelectorConfig::default();
    cfg.min_confidence_threshold = 0.0;
    cfg
}

#[tokio::test]
async fn selector_prefers_tool_with_matching_tags_caps() {
    let selector = IntelligentToolSelector::new(low_threshold_config());

    let a = mk_spec(
        "alpha",
        "Downloader tool",
        Some(mk_guide(vec!["download", "http"], vec!["fetch"], vec!["web"], "fast", 1)),
    );
    let b = mk_spec("beta", "Generic tool", None);

    selector.register_tool(a.clone()).await;
    selector.register_tool(b.clone()).await;

    let choices = selector.select_tools(&ctx("скачай страницу", UrgencyLevel::Normal)).await.unwrap();
    assert!(!choices.is_empty());
    // alpha should rank higher than beta
    let pos_alpha = choices.iter().position(|c| c.tool_name == "alpha").unwrap();
    let pos_beta = choices.iter().position(|c| c.tool_name == "beta").unwrap();
    assert!(pos_alpha < pos_beta, "alpha should outrank beta due to tags/caps match");
}

#[tokio::test]
async fn selector_accounts_for_latency_with_high_urgency() {
    let selector = IntelligentToolSelector::new(low_threshold_config());

    let fast = mk_spec("fast", "Fast tool", Some(mk_guide(vec![], vec![], vec![], "fast", 3)));
    let slow = mk_spec("slow", "Slow tool", Some(mk_guide(vec![], vec![], vec![], "slow", 3)));

    selector.register_tool(fast.clone()).await;
    selector.register_tool(slow.clone()).await;

    let choices = selector.select_tools(&ctx("latency test", UrgencyLevel::High)).await.unwrap();
    let pos_fast = choices.iter().position(|c| c.tool_name == "fast").unwrap();
    let pos_slow = choices.iter().position(|c| c.tool_name == "slow").unwrap();
    assert!(pos_fast < pos_slow, "fast latency should be preferred under high urgency");
}

#[tokio::test]
async fn selector_prefers_lower_risk() {
    let selector = IntelligentToolSelector::new(low_threshold_config());

    let low = mk_spec("lowrisk", "Low risk", Some(mk_guide(vec![], vec![], vec![], "fast", 1)));
    let high = mk_spec("highrisk", "High risk", Some(mk_guide(vec![], vec![], vec![], "fast", 5)));

    selector.register_tool(low.clone()).await;
    selector.register_tool(high.clone()).await;

    let choices = selector.select_tools(&ctx("do something", UrgencyLevel::Normal)).await.unwrap();
    let pos_low = choices.iter().position(|c| c.tool_name == "lowrisk").unwrap();
    let pos_high = choices.iter().position(|c| c.tool_name == "highrisk").unwrap();
    assert!(pos_low < pos_high, "low risk should have slight bonus");
}

#[tokio::test]
async fn selector_is_deterministic_for_same_input() {
    let selector = IntelligentToolSelector::new(SelectorConfig::default());

    let t1 = mk_spec("t1", "Tool one", Some(mk_guide(vec!["tag1"], vec!["cap1"], vec!["general"], "fast", 1)));
    let t2 = mk_spec("t2", "Tool two", Some(mk_guide(vec!["tag2"], vec!["cap2"], vec!["general"], "slow", 3)));
    selector.register_tool(t1.clone()).await;
    selector.register_tool(t2.clone()).await;

    let c1 = selector.select_tools(&ctx("some query with tag1", UrgencyLevel::Normal)).await.unwrap();
    let c2 = selector.select_tools(&ctx("some query with tag1", UrgencyLevel::Normal)).await.unwrap();
    assert_eq!(c1.len(), c2.len());
    // Compare order of tool names
    let names1: Vec<_> = c1.iter().map(|x| x.tool_name.clone()).collect();
    let names2: Vec<_> = c2.iter().map(|x| x.tool_name.clone()).collect();
    assert_eq!(names1, names2, "selection should be deterministic for same input");
}

#[tokio::test]
async fn selector_prefilters_shell_when_disallowed() {
    // Ensure environment disallows shell
    std::env::set_var("MAGRAY_ALLOW_SHELL", "0");
    let selector = IntelligentToolSelector::new(low_threshold_config());
    let shell = ToolSpec {
        name: "shell_exec".into(),
        description: "Shell".into(),
        usage: "shell_exec <cmd>".into(),
        examples: vec![],
        input_schema: "{}".into(),
        usage_guide: None,
        permissions: Some(tools::ToolPermissions { allow_shell: true, ..Default::default() }),
        supports_dry_run: true,
    };
    let other = mk_spec("other", "Generic", None);
    selector.register_tool(shell).await;
    selector.register_tool(other.clone()).await;
    let choices = selector.select_tools(&ctx("run command", UrgencyLevel::Normal)).await.unwrap();
    // shell_exec must not appear
    assert!(choices.iter().all(|c| c.tool_name != "shell_exec"));
}

#[tokio::test]
async fn selector_prefilters_network_when_allowlist_empty() {
    std::env::remove_var("MAGRAY_NET_ALLOW");
    let selector = IntelligentToolSelector::new(low_threshold_config());
    let web = ToolSpec {
        name: "web_fetch".into(),
        description: "Web".into(),
        usage: "web_fetch <url>".into(),
        examples: vec![],
        input_schema: "{}".into(),
        usage_guide: None,
        permissions: Some(tools::ToolPermissions { net_allowlist: vec!["example.com".into()], ..Default::default() }),
        supports_dry_run: true,
    };
    let other = mk_spec("other", "Generic", None);
    selector.register_tool(web).await;
    selector.register_tool(other.clone()).await;
    let choices = selector.select_tools(&ctx("скачай страницу", UrgencyLevel::Normal)).await.unwrap();
    assert!(choices.iter().all(|c| c.tool_name != "web_fetch"));
}

#[tokio::test]
async fn selector_prefilters_fs_when_sandbox_on_and_roots_insufficient() {
    std::env::set_var("MAGRAY_FS_SANDBOX", "1");
    std::env::set_var("MAGRAY_FS_ROOTS", "/allowed/root");
    let selector = IntelligentToolSelector::new(low_threshold_config());
    let needs_fs = ToolSpec {
        name: "file_write".into(),
        description: "File writer".into(),
        usage: "file_write".into(),
        examples: vec![],
        input_schema: "{}".into(),
        usage_guide: None,
        permissions: Some(tools::ToolPermissions { fs_write_roots: vec!["/other".into()], ..Default::default() }),
        supports_dry_run: true,
    };
    let ok = mk_spec("file_read", "Reader", None);
    selector.register_tool(needs_fs).await;
    selector.register_tool(ok.clone()).await;
    let choices = selector.select_tools(&ctx("write file", UrgencyLevel::Normal)).await.unwrap();
    assert!(choices.iter().all(|c| c.tool_name != "file_write"));
}