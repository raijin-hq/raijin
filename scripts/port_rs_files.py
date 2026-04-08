#!/usr/bin/env python3
"""Port .rs files: rename Zed crate references to Raijin equivalents."""
import os
import re

CRATE_DIRS = [
    'crates/raijin-dap-adapters',
    'crates/raijin-debug-adapter-extension',
    'crates/raijin-debugger-tools',
    'crates/raijin-debugger-ui',
    'crates/raijin-livekit-api',
    'crates/raijin-livekit-client',
    'crates/raijin-call',
    'crates/raijin-channel',
    'crates/raijin-collab',
    'crates/raijin-collab-ui',
    'crates/raijin-extension-cli',
    'crates/raijin-extension-host',
    'crates/raijin-extensions-ui',
    'crates/raijin-extension-api',
]

USE_REPLACEMENTS = [
    ('use gpui_tokio::', 'use inazuma_tokio::'),
    ('use gpui_platform::', 'use inazuma_platform::'),
    ('use gpui::', 'use inazuma::'),
    ('use ui_input::', 'use raijin_ui_input::'),
    ('use ui::', 'use raijin_ui::'),
    ('use theme_settings::', 'use raijin_theme_settings::'),
    ('use theme_extension::', 'use raijin_theme_extension::'),
    ('use theme::', 'use raijin_theme::'),
    ('use settings::', 'use inazuma_settings_framework::'),
    ('use workspace::', 'use raijin_workspace::'),
    ('use editor::', 'use raijin_editor::'),
    ('use collections::', 'use inazuma_collections::'),
    ('use util::', 'use inazuma_util::'),
    ('use util;', 'use inazuma_util;'),
    ('use project::', 'use raijin_project::'),
    ('use language_model::', 'use raijin_language_model::'),
    ('use language_extension::', 'use raijin_language_extension::'),
    ('use language::', 'use raijin_language::'),
    ('use client::', 'use raijin_client::'),
    ('use fs::', 'use raijin_fs::'),
    ('use picker::', 'use inazuma_picker::'),
    ('use menu::', 'use inazuma_menu::'),
    ('use zed_actions::', 'use raijin_actions::'),
    ('use fuzzy::', 'use inazuma_fuzzy::'),
    ('use dap_adapters::', 'use raijin_dap_adapters::'),
    ('use debugger_tools::', 'use raijin_debugger_tools::'),
    ('use debugger_ui::', 'use raijin_debugger_ui::'),
    ('use dap::', 'use raijin_dap::'),
    ('use task::', 'use raijin_task::'),
    ('use paths::', 'use raijin_paths::'),
    ('use text::', 'use raijin_text::'),
    ('use clock::', 'use inazuma_clock::'),
    ('use rpc::', 'use raijin_rpc::'),
    ('use db::', 'use raijin_db::'),
    ('use http_client_tls::', 'use raijin_http_client_tls::'),
    ('use http_client::', 'use raijin_http_client::'),
    ('use node_runtime::', 'use raijin_node_runtime::'),
    ('use livekit_api::', 'use raijin_livekit_api::'),
    ('use livekit_client::', 'use raijin_livekit_client::'),
    ('use feature_flags::', 'use raijin_feature_flags::'),
    ('use telemetry_events::', 'use raijin_telemetry_events::'),
    ('use telemetry::', 'use raijin_telemetry::'),
    ('use audio::', 'use raijin_audio::'),
    ('use release_channel::', 'use raijin_release_channel::'),
    ('use cloud_api_types::', 'use raijin_cloud_api_types::'),
    ('use extension_host::', 'use raijin_extension_host::'),
    ('use extension::', 'use raijin_extension::'),
    ('use file_icons::', 'use raijin_file_icons::'),
    ('use notifications::', 'use raijin_notifications::'),
    ('use command_palette_hooks::', 'use raijin_command_palette_hooks::'),
    ('use tasks_ui::', 'use raijin_tasks_ui::'),
    ('use terminal_view::', 'use raijin_terminal_view::'),
    ('use time_format::', 'use raijin_time_format::'),
    ('use title_bar::', 'use raijin_title_bar::'),
    ('use vim_mode_setting::', 'use raijin_vim_mode_setting::'),
    ('use call::', 'use raijin_call::'),
    ('use channel::', 'use raijin_channel::'),
    ('use collab_ui::', 'use raijin_collab_ui::'),
    ('use collab::', 'use raijin_collab::'),
    ('use multi_buffer::', 'use raijin_multi_buffer::'),
    ('use lsp::', 'use raijin_lsp::'),
    ('use remote_server::', 'use raijin_remote_server::'),
    ('use remote::', 'use raijin_remote::'),
    ('use settings_content::', 'use raijin_settings_content::'),
    ('use snippet_provider::', 'use raijin_snippet_provider::'),
    ('use reqwest_client::', 'use raijin_reqwest_client::'),
    ('use session::', 'use raijin_session::'),
    ('use worktree::', 'use raijin_worktree::'),
    ('use agent::', 'use raijin_agent::'),
    ('use buffer_diff::', 'use raijin_buffer_diff::'),
    ('use git_hosting_providers::', 'use raijin_git_hosting_providers::'),
    ('use git_ui::', 'use raijin_git_ui::'),
    ('use git::', 'use raijin_git::'),
    ('use file_finder::', 'use raijin_file_finder::'),
    ('use prompt_store::', 'use raijin_prompt_store::'),
    ('use recent_projects::', 'use raijin_recent_projects::'),
    ('use zlog::', 'use raijin_log::'),
    ('use ztracing::', 'use raijin_tracing::'),
    ('use assistant_text_thread::', 'use raijin_assistant_text_thread::'),
    ('use assistant_slash_command::', 'use raijin_assistant_slash_command::'),
]

INLINE_PATTERNS = [
    (r'(?<![a-zA-Z0-9_])gpui_tokio::', 'inazuma_tokio::'),
    (r'(?<![a-zA-Z0-9_])gpui_platform::', 'inazuma_platform::'),
    (r'(?<![a-zA-Z0-9_])gpui::', 'inazuma::'),
    (r'(?<![a-zA-Z0-9_])ui_input::', 'raijin_ui_input::'),
    (r'(?<![a-zA-Z0-9_])ui::', 'raijin_ui::'),
    (r'(?<![a-zA-Z0-9_])theme_settings::', 'raijin_theme_settings::'),
    (r'(?<![a-zA-Z0-9_])theme_extension::', 'raijin_theme_extension::'),
    (r'(?<![a-zA-Z0-9_])theme::', 'raijin_theme::'),
    (r'(?<![a-zA-Z0-9_])settings::', 'inazuma_settings_framework::'),
    (r'(?<![a-zA-Z0-9_])workspace::', 'raijin_workspace::'),
    (r'(?<![a-zA-Z0-9_])editor::', 'raijin_editor::'),
    (r'(?<![a-zA-Z0-9_])collections::', 'inazuma_collections::'),
    (r'(?<![a-zA-Z0-9_])util::', 'inazuma_util::'),
    (r'(?<![a-zA-Z0-9_])project::', 'raijin_project::'),
    (r'(?<![a-zA-Z0-9_])language_model::', 'raijin_language_model::'),
    (r'(?<![a-zA-Z0-9_])language_extension::', 'raijin_language_extension::'),
    (r'(?<![a-zA-Z0-9_])language::', 'raijin_language::'),
    (r'(?<![a-zA-Z0-9_])client::', 'raijin_client::'),
    (r'(?<![a-zA-Z0-9_])fs::', 'raijin_fs::'),
    (r'(?<![a-zA-Z0-9_])picker::', 'inazuma_picker::'),
    (r'(?<![a-zA-Z0-9_])menu::', 'inazuma_menu::'),
    (r'(?<![a-zA-Z0-9_])zed_actions::', 'raijin_actions::'),
    (r'(?<![a-zA-Z0-9_])fuzzy::', 'inazuma_fuzzy::'),
    (r'(?<![a-zA-Z0-9_])dap_adapters::', 'raijin_dap_adapters::'),
    (r'(?<![a-zA-Z0-9_])debugger_tools::', 'raijin_debugger_tools::'),
    (r'(?<![a-zA-Z0-9_])debugger_ui::', 'raijin_debugger_ui::'),
    (r'(?<![a-zA-Z0-9_])dap::', 'raijin_dap::'),
    (r'(?<![a-zA-Z0-9_])task::', 'raijin_task::'),
    (r'(?<![a-zA-Z0-9_])paths::', 'raijin_paths::'),
    (r'(?<![a-zA-Z0-9_])text::', 'raijin_text::'),
    (r'(?<![a-zA-Z0-9_])clock::', 'inazuma_clock::'),
    (r'(?<![a-zA-Z0-9_])rpc::', 'raijin_rpc::'),
    (r'(?<![a-zA-Z0-9_])db::', 'raijin_db::'),
    (r'(?<![a-zA-Z0-9_])http_client_tls::', 'raijin_http_client_tls::'),
    (r'(?<![a-zA-Z0-9_])http_client::', 'raijin_http_client::'),
    (r'(?<![a-zA-Z0-9_])node_runtime::', 'raijin_node_runtime::'),
    (r'(?<![a-zA-Z0-9_])livekit_api::', 'raijin_livekit_api::'),
    (r'(?<![a-zA-Z0-9_])livekit_client::', 'raijin_livekit_client::'),
    (r'(?<![a-zA-Z0-9_])feature_flags::', 'raijin_feature_flags::'),
    (r'(?<![a-zA-Z0-9_])telemetry_events::', 'raijin_telemetry_events::'),
    (r'(?<![a-zA-Z0-9_])telemetry::', 'raijin_telemetry::'),
    (r'(?<![a-zA-Z0-9_])audio::', 'raijin_audio::'),
    (r'(?<![a-zA-Z0-9_])release_channel::', 'raijin_release_channel::'),
    (r'(?<![a-zA-Z0-9_])cloud_api_types::', 'raijin_cloud_api_types::'),
    (r'(?<![a-zA-Z0-9_])extension_host::', 'raijin_extension_host::'),
    (r'(?<![a-zA-Z0-9_])extension::', 'raijin_extension::'),
    (r'(?<![a-zA-Z0-9_])file_icons::', 'raijin_file_icons::'),
    (r'(?<![a-zA-Z0-9_])notifications::', 'raijin_notifications::'),
    (r'(?<![a-zA-Z0-9_])command_palette_hooks::', 'raijin_command_palette_hooks::'),
    (r'(?<![a-zA-Z0-9_])tasks_ui::', 'raijin_tasks_ui::'),
    (r'(?<![a-zA-Z0-9_])terminal_view::', 'raijin_terminal_view::'),
    (r'(?<![a-zA-Z0-9_])time_format::', 'raijin_time_format::'),
    (r'(?<![a-zA-Z0-9_])title_bar::', 'raijin_title_bar::'),
    (r'(?<![a-zA-Z0-9_])vim_mode_setting::', 'raijin_vim_mode_setting::'),
    (r'(?<![a-zA-Z0-9_])call::', 'raijin_call::'),
    (r'(?<![a-zA-Z0-9_])channel::', 'raijin_channel::'),
    (r'(?<![a-zA-Z0-9_])collab_ui::', 'raijin_collab_ui::'),
    (r'(?<![a-zA-Z0-9_])collab::', 'raijin_collab::'),
    (r'(?<![a-zA-Z0-9_])multi_buffer::', 'raijin_multi_buffer::'),
    (r'(?<![a-zA-Z0-9_])lsp::', 'raijin_lsp::'),
    (r'(?<![a-zA-Z0-9_])remote_server::', 'raijin_remote_server::'),
    (r'(?<![a-zA-Z0-9_])remote::', 'raijin_remote::'),
    (r'(?<![a-zA-Z0-9_])session::', 'raijin_session::'),
    (r'(?<![a-zA-Z0-9_])worktree::', 'raijin_worktree::'),
    (r'(?<![a-zA-Z0-9_])agent::', 'raijin_agent::'),
    (r'(?<![a-zA-Z0-9_])buffer_diff::', 'raijin_buffer_diff::'),
    (r'(?<![a-zA-Z0-9_])git_hosting_providers::', 'raijin_git_hosting_providers::'),
    (r'(?<![a-zA-Z0-9_])git_ui::', 'raijin_git_ui::'),
    (r'(?<![a-zA-Z0-9_])git::', 'raijin_git::'),
    (r'(?<![a-zA-Z0-9_])zlog::', 'raijin_log::'),
    (r'(?<![a-zA-Z0-9_])ztracing::', 'raijin_tracing::'),
]

COMPILED_INLINE = [(re.compile(p), r) for p, r in INLINE_PATTERNS]

def process_content(content):
    for old, new in USE_REPLACEMENTS:
        content = content.replace(old, new)

    content = content.replace('actions!(zed,', 'actions!(raijin,')

    for pattern, replacement in COMPILED_INLINE:
        content = pattern.sub(replacement, content)

    # Fix over-replacements
    content = content.replace('std::raijin_fs::', 'std::fs::')
    content = content.replace('smol::raijin_fs::', 'smol::fs::')
    content = content.replace('tokio::raijin_fs::', 'tokio::fs::')
    content = content.replace('futures::raijin_task::', 'futures::task::')
    content = content.replace('std::raijin_task::', 'std::task::')
    content = content.replace('tokio::raijin_task::', 'tokio::task::')
    content = content.replace('core::raijin_task::', 'core::task::')
    content = content.replace('std::raijin_channel::', 'std::channel::')
    content = content.replace('std::raijin_text::', 'std::text::')
    content = content.replace('core::raijin_text::', 'core::text::')
    content = re.sub(r'\bself::raijin_', 'self::', content)
    content = re.sub(r'\bsuper::raijin_', 'super::', content)
    content = re.sub(r'\bcrate::raijin_', 'crate::', content)
    content = re.sub(r'\bself::inazuma_', 'self::', content)
    content = re.sub(r'\bsuper::inazuma_', 'super::', content)
    content = re.sub(r'\bcrate::inazuma_', 'crate::', content)
    content = content.replace('raijin_raijin_', 'raijin_')
    content = content.replace('inazuma_inazuma_', 'inazuma_')

    return content

files = []
for d in CRATE_DIRS:
    for root, dirs, fnames in os.walk(d):
        for f in fnames:
            if f.endswith('.rs'):
                files.append(os.path.join(root, f))

count = 0
for fpath in files:
    with open(fpath, 'r') as f:
        content = f.read()

    new_content = process_content(content)

    if new_content != content:
        with open(fpath, 'w') as f:
            f.write(new_content)
        count += 1

print(f'Processed {len(files)} files, modified {count}')
