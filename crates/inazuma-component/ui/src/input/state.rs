//! A text input field that allows the user to enter text.
//!
//! Based on the `Input` example from the `gpui` crate.
//! https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/input.rs
use anyhow::Result;
use inazuma::{
    Action, App, AppContext, Bounds, Context, Entity, EntityInputHandler, EventEmitter, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, KeyBinding, KeyDownEvent, MouseMoveEvent,
    ParentElement as _, Pixels, Point, Render, ScrollHandle, ShapedLine, SharedString, Styled as _,
    Subscription, Task, Window, actions, div, point, prelude::FluentBuilder as _, px,
};
use inazuma::{Half, TextAlign};
use ropey::{Rope, RopeSlice};
use serde::Deserialize;
use std::ops::Range;
use std::rc::Rc;
use inazuma_sum_tree::Bias;

use super::{
    DisplayMap, blink_cursor::BlinkCursor, change::Change, element::TextElement,
    mask_pattern::MaskPattern, mode::InputMode, number_input,
};
use crate::Size;
use crate::actions::{SelectDown, SelectLeft, SelectRight, SelectUp};
use crate::highlighter::DiagnosticSet;
#[cfg(not(target_family = "wasm"))]
use crate::highlighter::LanguageRegistry;
use crate::input::{
    HoverDefinition, InlineCompletion, Lsp, Position, RopeExt as _, Selection,
    display_map::LineLayout,
    popovers::{ContextMenu, DiagnosticPopover, HoverPopover, MouseContextMenu},
    search::{self, SearchPanel},
};
use crate::{Root, history::History};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = input, no_json)]
pub struct Enter {
    /// Is confirm with secondary.
    pub secondary: bool,
}

actions!(
    input,
    [
        Backspace,
        Delete,
        DeleteToBeginningOfLine,
        DeleteToEndOfLine,
        DeleteToPreviousWordStart,
        DeleteToNextWordEnd,
        Indent,
        Outdent,
        IndentInline,
        OutdentInline,
        MoveUp,
        MoveDown,
        MoveLeft,
        MoveRight,
        MoveHome,
        MoveEnd,
        MovePageUp,
        MovePageDown,
        SelectAll,
        SelectToStartOfLine,
        SelectToEndOfLine,
        SelectToStart,
        SelectToEnd,
        SelectToPreviousWordStart,
        SelectToNextWordEnd,
        ShowCharacterPalette,
        Copy,
        Cut,
        Paste,
        Undo,
        Redo,
        MoveToStartOfLine,
        MoveToEndOfLine,
        MoveToStart,
        MoveToEnd,
        MoveToPreviousWord,
        MoveToNextWord,
        Escape,
        ToggleCodeActions,
        Search,
        GoToDefinition,
    ]
);

#[derive(Clone)]
pub enum InputEvent {
    Change,
    PressEnter { secondary: bool },
    Focus,
    Blur,
    /// Emitted when Up is pressed on the first row (for command history navigation).
    HistoryUp,
    /// Emitted when Down is pressed on the last row (for command history navigation).
    HistoryDown,
}

pub(super) const CONTEXT: &str = "Input";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some(CONTEXT)),
        KeyBinding::new("shift-backspace", Backspace, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-backspace", Backspace, Some(CONTEXT)),
        KeyBinding::new("delete", Delete, Some(CONTEXT)),
        KeyBinding::new("shift-delete", Delete, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-backspace", DeleteToBeginningOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-delete", DeleteToEndOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-backspace", DeleteToPreviousWordStart, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-backspace", DeleteToPreviousWordStart, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-delete", DeleteToNextWordEnd, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-delete", DeleteToNextWordEnd, Some(CONTEXT)),
        KeyBinding::new("enter", Enter { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("shift-enter", Enter { secondary: true }, Some(CONTEXT)),
        KeyBinding::new("secondary-enter", Enter { secondary: true }, Some(CONTEXT)),
        KeyBinding::new("escape", Escape, Some(CONTEXT)),
        KeyBinding::new("up", MoveUp, Some(CONTEXT)),
        KeyBinding::new("down", MoveDown, Some(CONTEXT)),
        KeyBinding::new("left", MoveLeft, Some(CONTEXT)),
        KeyBinding::new("right", MoveRight, Some(CONTEXT)),
        KeyBinding::new("pageup", MovePageUp, Some(CONTEXT)),
        KeyBinding::new("pagedown", MovePageDown, Some(CONTEXT)),
        KeyBinding::new("tab", IndentInline, Some(CONTEXT)),
        KeyBinding::new("shift-tab", OutdentInline, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-]", Indent, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-]", Indent, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-[", Outdent, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-[", Outdent, Some(CONTEXT)),
        KeyBinding::new("shift-left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("shift-right", SelectRight, Some(CONTEXT)),
        KeyBinding::new("shift-up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("shift-down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("home", MoveHome, Some(CONTEXT)),
        KeyBinding::new("end", MoveEnd, Some(CONTEXT)),
        KeyBinding::new("shift-home", SelectToStartOfLine, Some(CONTEXT)),
        KeyBinding::new("shift-end", SelectToEndOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-shift-a", SelectToStartOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-shift-e", SelectToEndOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("shift-cmd-left", SelectToStartOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("shift-cmd-right", SelectToEndOfLine, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-shift-left", SelectToPreviousWordStart, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-left", SelectToPreviousWordStart, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-shift-right", SelectToNextWordEnd, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-right", SelectToNextWordEnd, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-a", SelectAll, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", SelectAll, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-x", Cut, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-a", MoveHome, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-left", MoveHome, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-e", MoveEnd, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-right", MoveEnd, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-z", Undo, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-z", Redo, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-up", MoveToStart, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-down", MoveToEnd, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-left", MoveToPreviousWord, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-right", MoveToNextWord, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-left", MoveToPreviousWord, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-right", MoveToNextWord, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-up", SelectToStart, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-down", SelectToEnd, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-z", Undo, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-y", Redo, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-.", ToggleCodeActions, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-.", ToggleCodeActions, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-f", Search, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-f", Search, Some(CONTEXT)),
    ]);

    search::init(cx);
    number_input::init(cx);
}

/// Whitespace indicators for rendering spaces and tabs.
#[derive(Clone, Default)]
pub(crate) struct WhitespaceIndicators {
    /// Shaped line for space character indicator (•)
    pub(crate) space: ShapedLine,
    /// Shaped line for tab character indicator (→)
    pub(crate) tab: ShapedLine,
}

#[derive(Clone)]
pub(super) struct LastLayout {
    /// The visible range (no wrap) of lines in the viewport, the value is row (0-based) index.
    /// This is the buffer line range that encompasses all visible lines.
    pub(super) visible_range: Range<usize>,
    /// The list of visible buffer line indices (excludes hidden/folded lines).
    /// Parallel to `lines`: `visible_buffer_lines[i]` is the buffer line index of `lines[i]`.
    pub(super) visible_buffer_lines: Vec<usize>,
    /// Byte offset of each visible buffer line in the Rope (parallel to visible_buffer_lines/lines).
    pub(super) visible_line_byte_offsets: Vec<usize>,
    /// The first visible line top position in scroll viewport.
    pub(super) visible_top: Pixels,
    /// The range of byte offset of the visible lines.
    pub(super) visible_range_offset: Range<usize>,
    /// The last layout lines (Only have visible lines, no empty entries for hidden lines).
    pub(super) lines: Rc<Vec<LineLayout>>,
    /// The line_height of text layout, this will change will InputElement painted.
    pub(super) line_height: Pixels,
    /// The wrap width of text layout, this will change will InputElement painted.
    pub(super) wrap_width: Option<Pixels>,
    /// The line number area width of text layout, if not line number, this will be 0px.
    pub(super) line_number_width: Pixels,
    /// The cursor position (top, left) in pixels.
    pub(super) cursor_bounds: Option<Bounds<Pixels>>,
    /// The text align of the text layout.
    pub(super) text_align: TextAlign,
    /// The content width of the text layout.
    pub(super) content_width: Pixels,
}

impl LastLayout {
    /// Get the line layout for the given buffer row (0-based).
    ///
    /// Uses binary search on `visible_buffer_lines` to find the line.
    /// Returns None if the row is not visible (out of range or folded).
    pub(crate) fn line(&self, row: usize) -> Option<&LineLayout> {
        let pos = self.visible_buffer_lines.binary_search(&row).ok()?;
        self.lines.get(pos)
    }

    /// Get the alignment offset for the given line width.
    pub(super) fn alignment_offset(&self, line_width: Pixels) -> Pixels {
        match self.text_align {
            TextAlign::Left => px(0.),
            TextAlign::Center => (self.content_width - line_width).half().max(px(0.)),
            TextAlign::Right => (self.content_width - line_width).max(px(0.)),
        }
    }
}

/// InputState to keep editing state of the [`super::Input`].
pub struct InputState {
    pub(super) focus_handle: FocusHandle,
    pub(super) mode: InputMode,
    pub(super) text: Rope,
    pub(super) display_map: DisplayMap,
    pub(super) history: History<Change>,
    pub(super) blink_cursor: Entity<BlinkCursor>,
    pub(super) loading: bool,
    /// Range in UTF-8 length for the selected text.
    ///
    /// - "Hello 世界💝" = 16
    /// - "💝" = 4
    pub(super) selected_range: Selection,
    pub(super) search_panel: Option<Entity<SearchPanel>>,
    pub(super) searchable: bool,
    /// Range for save the selected word, use to keep word range when drag move.
    pub(super) selected_word_range: Option<Selection>,
    pub(super) selection_reversed: bool,
    /// The marked range is the temporary insert text on IME typing.
    pub(super) ime_marked_range: Option<Selection>,
    pub(super) last_layout: Option<LastLayout>,
    pub(super) last_cursor: Option<usize>,
    /// The input container bounds
    pub(super) input_bounds: Bounds<Pixels>,
    /// The text bounds
    pub(super) last_bounds: Option<Bounds<Pixels>>,
    pub(super) last_selected_range: Option<Selection>,
    pub(super) selecting: bool,
    pub(super) size: Size,
    pub(super) disabled: bool,
    pub(super) masked: bool,
    pub(super) clean_on_escape: bool,
    pub(super) soft_wrap: bool,
    pub(super) show_whitespaces: bool,
    pub(super) pattern: Option<regex::Regex>,
    pub(super) validate: Option<Box<dyn Fn(&str, &mut Context<Self>) -> bool + 'static>>,
    pub(crate) scroll_handle: ScrollHandle,
    /// The deferred scroll offset to apply on next layout.
    pub(crate) deferred_scroll_offset: Option<Point<Pixels>>,
    /// The size of the scrollable content.
    pub(crate) scroll_size: inazuma::Size<Pixels>,
    pub(super) text_align: TextAlign,

    /// The mask pattern for formatting the input text
    pub(crate) mask_pattern: MaskPattern,
    pub(super) placeholder: SharedString,

    /// Popover
    pub(super) diagnostic_popover: Option<Entity<DiagnosticPopover>>,
    /// Completion/CodeAction context menu
    pub(super) context_menu: Option<ContextMenu>,
    pub(super) mouse_context_menu: Entity<MouseContextMenu>,
    /// A flag to indicate if we are currently inserting a completion item.
    pub completion_inserting: bool,
    pub(super) hover_popover: Option<Entity<HoverPopover>>,
    /// The LSP definitions locations for "Go to Definition" feature.
    pub(super) hover_definition: HoverDefinition,

    pub lsp: Lsp,

    /// Overlay highlights for text coloring (e.g. completion preview, command validation).
    /// These are merged on top of syntax highlighting during rendering.
    pub overlay_highlights: Vec<(std::ops::Range<usize>, inazuma::HighlightStyle)>,

    /// Byte range of text that was inserted by a completion (menu or inline ghost text).
    /// Used by the highlight system to color completion-inserted text differently.
    /// Cleared automatically when the user types manually.
    pub completion_inserted_range: Option<std::ops::Range<usize>>,

    /// A flag to indicate if we have a pending update to the text.
    ///
    /// If true, will call some update (for example LSP, Syntax Highlight) before render.
    _pending_update: bool,
    /// A flag to indicate if we should ignore the next completion event.
    pub(super) silent_replace_text: bool,

    /// To remember the horizontal column (x-coordinate) of the cursor position for keep column for move up/down.
    ///
    /// The first element is the x-coordinate (Pixels), preferred to use this.
    /// The second element is the column (usize), fallback to use this.
    pub(super) preferred_column: Option<(Pixels, usize)>,
    _subscriptions: Vec<Subscription>,

    pub(super) _context_menu_task: Task<Result<()>>,
    pub(super) inline_completion: InlineCompletion,

    /// Auto-closing bracket/quote configuration.
    pub(super) auto_pairs: super::auto_pairs::AutoPairConfig,
    /// Flag set during paste operations to suppress auto-closing.
    pub(super) is_pasting: bool,
}

impl EventEmitter<InputEvent> for InputState {}

impl InputState {
    /// Create a Input state with default [`InputMode::SingleLine`] mode.
    ///
    /// See also: [`Self::multi_line`], [`Self::auto_grow`] to set other mode.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle().tab_stop(true);
        let blink_cursor = cx.new(|_| BlinkCursor::new());
        let history = History::new().group_interval(std::time::Duration::from_secs(1));

        let _subscriptions = vec![
            // Observe the blink cursor to repaint the view when it changes.
            cx.observe(&blink_cursor, |_, _, cx| cx.notify()),
            // Blink the cursor when the window is active, pause when it's not.
            cx.observe_window_activation(window, |input, window, cx| {
                if window.is_window_active() {
                    let focus_handle = input.focus_handle.clone();
                    if focus_handle.is_focused(window) {
                        input.blink_cursor.update(cx, |blink_cursor, cx| {
                            blink_cursor.start(cx);
                        });
                    }
                }
            }),
            cx.on_focus(&focus_handle, window, Self::on_focus),
            cx.on_blur(&focus_handle, window, Self::on_blur),
        ];

        let text_style = window.text_style();
        let mouse_context_menu = MouseContextMenu::new(cx.entity(), window, cx);

        Self {
            focus_handle: focus_handle.clone(),
            text: "".into(),
            display_map: DisplayMap::new(text_style.font(), window.rem_size(), None),
            blink_cursor,
            history,
            selected_range: Selection::default(),
            search_panel: None,
            searchable: false,
            selected_word_range: None,
            selection_reversed: false,
            ime_marked_range: None,
            input_bounds: Bounds::default(),
            selecting: false,
            disabled: false,
            masked: false,
            clean_on_escape: false,
            soft_wrap: true,
            show_whitespaces: false,
            loading: false,
            pattern: None,
            validate: None,
            mode: InputMode::default(),
            last_layout: None,
            last_bounds: None,
            last_selected_range: None,
            last_cursor: None,
            scroll_handle: ScrollHandle::new(),
            scroll_size: inazuma::size(px(0.), px(0.)),
            deferred_scroll_offset: None,
            preferred_column: None,
            placeholder: SharedString::default(),
            mask_pattern: MaskPattern::default(),
            text_align: TextAlign::Left,
            lsp: Lsp::default(),
            overlay_highlights: Vec::new(),
            completion_inserted_range: None,
            diagnostic_popover: None,
            context_menu: None,
            mouse_context_menu,
            completion_inserting: false,
            hover_popover: None,
            hover_definition: HoverDefinition::default(),
            silent_replace_text: false,
            size: Size::default(),
            _subscriptions,
            _context_menu_task: Task::ready(Ok(())),
            _pending_update: false,
            inline_completion: InlineCompletion::default(),
            auto_pairs: super::auto_pairs::AutoPairConfig::default(),
            is_pasting: false,
        }
    }

    /// Set Input to use multi line mode.
    ///
    /// Default rows is 2.
    pub fn multi_line(mut self, multi_line: bool) -> Self {
        self.mode = self.mode.multi_line(multi_line);
        self
    }

    /// Set Input to use [`InputMode::AutoGrow`] mode with min, max rows limit.
    pub fn auto_grow(mut self, min_rows: usize, max_rows: usize) -> Self {
        self.mode = InputMode::auto_grow(min_rows, max_rows);
        self
    }

    /// Set Input to use [`InputMode::CodeEditor`] mode.
    ///
    /// Default options:
    ///
    /// - line_number: true
    /// - tab_size: 2
    /// - hard_tabs: false
    /// - height: 100%
    /// - multi_line: true
    /// - indent_guides: true
    ///
    /// If `highlighter` is None, will use the default highlighter.
    ///
    /// Code Editor aim for help used to simple code editing or display, not a full-featured code editor.
    ///
    /// ## Features
    ///
    /// - Syntax Highlighting
    /// - Auto Indent
    /// - Line Number
    /// - Large Text support, up to 50K lines.
    pub fn code_editor(mut self, language: impl Into<SharedString>) -> Self {
        let language: SharedString = language.into();
        self.mode = InputMode::code_editor(language);
        self.searchable = true;
        self
    }

    /// Set Input to use [`InputMode::ShellEditor`] mode — auto-growing
    /// multi-line input with syntax highlighting, without line numbers,
    /// indent guides, or code folding. Designed for terminal command input.
    pub fn shell_editor(
        mut self,
        language: impl Into<SharedString>,
        min_rows: usize,
        max_rows: usize,
    ) -> Self {
        self.mode = InputMode::shell_editor(language, min_rows, max_rows);
        self.soft_wrap = true;
        self
    }

    /// Update the syntax highlighting language for ShellEditor mode.
    pub fn set_shell_language(
        &mut self,
        language: impl Into<SharedString>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.mode.set_language(language);
    }

    /// Set auto-closing bracket/quote pairs configuration.
    pub fn auto_pairs(mut self, config: super::auto_pairs::AutoPairConfig) -> Self {
        self.auto_pairs = config;
        self
    }

    /// Set this input is searchable, default is false (Default true for Code Editor).
    pub fn searchable(mut self, searchable: bool) -> Self {
        debug_assert!(self.mode.is_multi_line());
        self.searchable = searchable;
        self
    }

    /// Set placeholder
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set enable/disable code folding, only for [`InputMode::CodeEditor`] mode.
    ///
    /// Default: true
    pub fn folding(mut self, folding: bool) -> Self {
        debug_assert!(self.mode.is_code_editor());
        if let InputMode::CodeEditor { folding: f, .. } = &mut self.mode {
            *f = folding;
        }
        self
    }

    /// Set code folding at runtime, only for [`InputMode::CodeEditor`] mode.
    ///
    /// When disabling, all existing folds are cleared.
    pub fn set_folding(&mut self, folding: bool, _: &mut Window, cx: &mut Context<Self>) {
        debug_assert!(self.mode.is_code_editor());
        if let InputMode::CodeEditor { folding: f, .. } = &mut self.mode {
            *f = folding;
        }
        if !folding {
            self.display_map.clear_folds();
        }
        cx.notify();
    }

    /// Set enable/disable line number, only for [`InputMode::CodeEditor`] mode.
    pub fn line_number(mut self, line_number: bool) -> Self {
        debug_assert!(self.mode.is_code_editor() && self.mode.is_multi_line());
        if let InputMode::CodeEditor { line_number: l, .. } = &mut self.mode {
            *l = line_number;
        }
        self
    }

    /// Set line number, only for [`InputMode::CodeEditor`] mode.
    pub fn set_line_number(&mut self, line_number: bool, _: &mut Window, cx: &mut Context<Self>) {
        debug_assert!(self.mode.is_code_editor() && self.mode.is_multi_line());
        if let InputMode::CodeEditor { line_number: l, .. } = &mut self.mode {
            *l = line_number;
        }
        cx.notify();
    }

    /// Set the number of rows for the multi-line Textarea.
    ///
    /// This is only used when `multi_line` is set to true.
    ///
    /// default: 2
    pub fn rows(mut self, rows: usize) -> Self {
        match &mut self.mode {
            InputMode::PlainText { rows: r, .. } | InputMode::CodeEditor { rows: r, .. } => {
                *r = rows
            }
            InputMode::AutoGrow {
                max_rows: max_r,
                rows: r,
                ..
            } => {
                *r = rows;
                *max_r = rows;
            }
            InputMode::ShellEditor {
                max_rows: max_r,
                rows: r,
                ..
            } => {
                *r = rows;
                *max_r = rows;
            }
        }
        self
    }

    /// Set highlighter language for for [`InputMode::CodeEditor`] mode.
    pub fn set_highlighter(
        &mut self,
        new_language: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        match &mut self.mode {
            InputMode::CodeEditor {
                language,
                highlighter,
                parse_task,
                ..
            } => {
                *language = new_language.into();
                *highlighter.borrow_mut() = None;
                parse_task.borrow_mut().take();
            }
            _ => {}
        }
        cx.notify();
    }

    fn reset_highlighter(&mut self, cx: &mut Context<Self>) {
        match &mut self.mode {
            InputMode::CodeEditor {
                highlighter,
                parse_task,
                ..
            } => {
                *highlighter.borrow_mut() = None;
                parse_task.borrow_mut().take();
            }
            _ => {}
        }
        cx.notify();
    }

    #[inline]
    pub fn diagnostics(&self) -> Option<&DiagnosticSet> {
        self.mode.diagnostics()
    }

    #[inline]
    pub fn diagnostics_mut(&mut self) -> Option<&mut DiagnosticSet> {
        self.mode.diagnostics_mut()
    }

    /// Set placeholder
    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    /// Find which line and sub-line the given offset belongs to, along with the position within that sub-line.
    ///
    /// Returns:
    ///
    /// - The index of the line (zero-based) containing the offset.
    /// - The index of the sub-line (zero-based) within the line containing the offset.
    /// - The position of the offset.
    pub(super) fn line_and_position_for_offset(
        &self,
        offset: usize,
    ) -> (usize, usize, Option<Point<Pixels>>) {
        let Some(last_layout) = &self.last_layout else {
            return (0, 0, None);
        };
        let line_height = last_layout.line_height;

        let mut y_offset = last_layout.visible_top;
        for (vi, line) in last_layout.lines.iter().enumerate() {
            let prev_lines_offset = last_layout.visible_line_byte_offsets[vi];
            let local_offset = offset.saturating_sub(prev_lines_offset);
            if let Some(pos) = line.position_for_index(local_offset, last_layout) {
                let sub_line_index = (pos.y / line_height) as usize;
                let adjusted_pos = point(pos.x + last_layout.line_number_width, pos.y + y_offset);
                return (vi, sub_line_index, Some(adjusted_pos));
            }

            y_offset += line.size(line_height).height;
        }
        (0, 0, None)
    }

    /// Set the text of the input field.
    ///
    /// And the selection_range will be reset to 0..0.
    pub fn set_value(
        &mut self,
        value: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.history.ignore = true;
        self.replace_text(value, window, cx);
        self.history.ignore = false;

        // Ensure cursor to start when set text
        if self.mode.is_single_line() {
            self.selected_range = (self.text.len()..self.text.len()).into();
        } else {
            self.selected_range.clear();
        }

        if self.mode.is_code_editor() {
            self._pending_update = true;
            self.lsp.reset();
        }

        // Move scroll to top
        self.scroll_handle.set_offset(point(px(0.), px(0.)));

        cx.notify();
    }

    /// Insert text at the current cursor position.
    ///
    /// And the cursor will be moved to the end of inserted text.
    pub fn insert(
        &mut self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let was_disabled = self.disabled;
        self.disabled = false;
        let text: SharedString = text.into();
        let range_utf16 = self.range_to_utf16(&(self.cursor()..self.cursor()));
        self.replace_text_in_range_silent(Some(range_utf16), &text, window, cx);
        self.selected_range = (self.selected_range.end..self.selected_range.end).into();
        self.disabled = was_disabled;
    }

    /// Replace text at the current cursor position.
    ///
    /// And the cursor will be moved to the end of replaced text.
    pub fn replace(
        &mut self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let was_disabled = self.disabled;
        self.disabled = false;
        let text: SharedString = text.into();
        self.replace_text_in_range_silent(None, &text, window, cx);
        self.selected_range = (self.selected_range.end..self.selected_range.end).into();
        self.disabled = was_disabled;
    }

    pub(super) fn replace_text(
        &mut self,
        text: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let was_disabled = self.disabled;
        self.disabled = false;
        let text: SharedString = text.into();
        let range = 0..self.text.chars().map(|c| c.len_utf16()).sum();
        self.replace_text_in_range_silent(Some(range), &text, window, cx);
        self.reset_highlighter(cx);
        self.disabled = was_disabled;
    }

    /// Set with disabled mode.
    ///
    /// See also: [`Self::set_disabled`], [`Self::is_disabled`].
    #[allow(unused)]
    pub(crate) fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set with password masked state.
    ///
    /// Only for [`InputMode::SingleLine`] mode.
    pub fn masked(mut self, masked: bool) -> Self {
        debug_assert!(self.mode.is_single_line());
        self.masked = masked;
        self
    }

    /// Set the password masked state of the input field.
    ///
    /// Only for [`InputMode::SingleLine`] mode.
    pub fn set_masked(&mut self, masked: bool, _: &mut Window, cx: &mut Context<Self>) {
        debug_assert!(self.mode.is_single_line());
        self.masked = masked;
        cx.notify();
    }

    /// Set true to clear the input by pressing Escape key.
    pub fn clean_on_escape(mut self) -> Self {
        self.clean_on_escape = true;
        self
    }

    /// Set the soft wrap mode for multi-line input, default is true.
    pub fn soft_wrap(mut self, wrap: bool) -> Self {
        debug_assert!(self.mode.is_multi_line());
        self.soft_wrap = wrap;
        self
    }

    /// Set whether to show whitespace characters.
    pub fn show_whitespaces(mut self, show: bool) -> Self {
        self.show_whitespaces = show;
        self
    }

    /// Update the soft wrap mode for multi-line input, default is true.
    pub fn set_soft_wrap(&mut self, wrap: bool, _: &mut Window, cx: &mut Context<Self>) {
        debug_assert!(self.mode.is_multi_line());
        self.soft_wrap = wrap;
        if wrap {
            let wrap_width = self
                .last_layout
                .as_ref()
                .and_then(|b| b.wrap_width)
                .unwrap_or(self.input_bounds.size.width);

            self.display_map.on_layout_changed(Some(wrap_width), cx);

            // Reset scroll to left 0
            let mut offset = self.scroll_handle.offset();
            offset.x = px(0.);
            self.scroll_handle.set_offset(offset);
        } else {
            self.display_map.on_layout_changed(None, cx);
        }
        cx.notify();
    }

    /// Update whether to show whitespace characters.
    pub fn set_show_whitespaces(&mut self, show: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.show_whitespaces = show;
        cx.notify();
    }

    /// Set the regular expression pattern of the input field.
    ///
    /// Only for [`InputMode::SingleLine`] mode.
    pub fn pattern(mut self, pattern: regex::Regex) -> Self {
        debug_assert!(self.mode.is_single_line());
        self.pattern = Some(pattern);
        self
    }

    /// Set the regular expression pattern of the input field with reference.
    ///
    /// Only for [`InputMode::SingleLine`] mode.
    pub fn set_pattern(
        &mut self,
        pattern: regex::Regex,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        debug_assert!(self.mode.is_single_line());
        self.pattern = Some(pattern);
    }

    /// Set the validation function of the input field.
    ///
    /// Only for [`InputMode::SingleLine`] mode.
    pub fn validate(mut self, f: impl Fn(&str, &mut Context<Self>) -> bool + 'static) -> Self {
        debug_assert!(self.mode.is_single_line());
        self.validate = Some(Box::new(f));
        self
    }

    /// Set true to show spinner at the input right.
    ///
    /// Only for [`InputMode::SingleLine`] mode.
    pub fn set_loading(&mut self, loading: bool, _: &mut Window, cx: &mut Context<Self>) {
        debug_assert!(self.mode.is_single_line());
        self.loading = loading;
        cx.notify();
    }

    /// Set the default value of the input field.
    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        let text: SharedString = value.into();
        self.text = Rope::from(text.as_str());
        if let Some(diagnostics) = self.mode.diagnostics_mut() {
            diagnostics.reset(&self.text)
        }
        // Note: We can't call display_map.set_text here because it needs cx.
        // The text will be set during prepare_if_need in element.rs
        self._pending_update = true;
        self
    }

    /// Return the value of the input field.
    pub fn value(&self) -> SharedString {
        SharedString::new(self.text.to_string())
    }

    /// Return the portion of the value within the input field that
    /// is selected by the user
    pub fn selected_value(&self) -> SharedString {
        SharedString::new(self.selected_text().to_string())
    }

    /// Return the value without mask.
    pub fn unmask_value(&self) -> SharedString {
        self.mask_pattern.unmask(&self.text.to_string()).into()
    }

    /// Return the text [`Rope`] of the input field.
    pub fn text(&self) -> &Rope {
        &self.text
    }

    /// Return the (0-based) [`Position`] of the cursor.
    pub fn cursor_position(&self) -> Position {
        let offset = self.cursor();
        self.text.offset_to_position(offset)
    }

    /// Set (0-based) [`Position`] of the cursor.
    ///
    /// This will move the cursor to the specified line and column, and update the selection range.
    pub fn set_cursor_position(
        &mut self,
        position: impl Into<Position>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let position: Position = position.into();
        let offset = self.text.position_to_offset(&position);

        self.move_to(offset, None, cx);
        self.update_preferred_column();
        self.focus(window, cx);
    }

    /// Focus the input field.
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle.focus(window, cx);
        self.blink_cursor.update(cx, |cursor, cx| {
            cursor.start(cx);
        });
    }

    /// Get byte offset of the cursor.
    ///
    /// The offset is the UTF-8 offset.
    pub fn cursor(&self) -> usize {
        if let Some(ime_marked_range) = &self.ime_marked_range {
            return ime_marked_range.end;
        }

        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    /// Select the text from the current cursor position to the given offset.
    ///
    /// The offset is the UTF-8 offset.
    ///
    /// Ensure the offset use self.next_boundary or self.previous_boundary to get the correct offset.
    pub(crate) fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.clear_inline_completion(cx);

        let offset = offset.clamp(0, self.text.len());
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };

        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = (self.selected_range.end..self.selected_range.start).into();
        }

        // Ensure keep word selected range
        if let Some(word_range) = self.selected_word_range.as_ref() {
            if self.selected_range.start > word_range.start {
                self.selected_range.start = word_range.start;
            }
            if self.selected_range.end < word_range.end {
                self.selected_range.end = word_range.end;
            }
        }
        if self.selected_range.is_empty() {
            self.update_preferred_column();
        }
        cx.notify()
    }

    /// Unselects the currently selected text.
    pub fn unselect(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.cursor();
        self.selected_range = (offset..offset).into();
        cx.notify()
    }

    /// If offset falls on a hidden (folded) line, clamp backward to the end of
    /// the fold header line (last visible position before the fold).
    fn clamp_offset_to_visible_backward(&self, offset: usize) -> usize {
        let line = self.text.offset_to_point(offset).row;
        if self.display_map.is_buffer_line_hidden(line) {
            for fold in self.display_map.folded_ranges() {
                if line > fold.start_line && line <= fold.end_line {
                    return self.text.line_end_offset(fold.start_line);
                }
            }
        }
        offset
    }

    /// If offset falls on a hidden (folded) line, clamp forward to the start of
    /// the fold end line (first visible position after the fold).
    fn clamp_offset_to_visible_forward(&self, offset: usize) -> usize {
        let line = self.text.offset_to_point(offset).row;
        if self.display_map.is_buffer_line_hidden(line) {
            for fold in self.display_map.folded_ranges() {
                if line > fold.start_line && line <= fold.end_line {
                    return self.text.line_start_offset(fold.end_line);
                }
            }
        }
        offset
    }

    pub(super) fn previous_boundary(&self, offset: usize) -> usize {
        let mut offset = self.text.clip_offset(offset.saturating_sub(1), Bias::Left);
        if let Some(ch) = self.text.char_at(offset) {
            if ch == '\r' {
                offset -= 1;
            }
        }

        self.clamp_offset_to_visible_backward(offset)
    }

    pub(super) fn next_boundary(&self, offset: usize) -> usize {
        let mut offset = self.text.clip_offset(offset + 1, Bias::Right);
        if let Some(ch) = self.text.char_at(offset) {
            if ch == '\r' {
                offset += 1;
            }
        }

        self.clamp_offset_to_visible_forward(offset)
    }

    /// Returns the true to let InputElement to render cursor, when Input is focused and current BlinkCursor is visible.
    pub(crate) fn show_cursor(&self, window: &Window, cx: &App) -> bool {
        (self.focus_handle.is_focused(window) || self.is_context_menu_open(cx))
            && !self.disabled
            && self.blink_cursor.read(cx).visible()
            && window.is_window_active()
    }

    fn on_focus(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.blink_cursor.update(cx, |cursor, cx| {
            cursor.start(cx);
        });
        cx.emit(InputEvent::Focus);
    }

    fn on_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_context_menu_open(cx) {
            return;
        }

        // NOTE: Do not cancel select, when blur.
        // Because maybe user want to copy the selected text by AppMenuBar (will take focus handle).

        self.hover_popover = None;
        self.diagnostic_popover = None;
        self.context_menu = None;
        self.clear_inline_completion(cx);
        self.blink_cursor.update(cx, |cursor, cx| {
            cursor.stop(cx);
        });
        Root::update(window, cx, |root, _, _| {
            root.focused_input = None;
        });
        cx.emit(InputEvent::Blur);
        cx.notify();
    }

    pub(super) fn pause_blink_cursor(&mut self, cx: &mut Context<Self>) {
        self.blink_cursor.update(cx, |cursor, cx| {
            cursor.pause(cx);
        });
    }

    pub(super) fn on_key_down(&mut self, _: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.pause_blink_cursor(cx);
    }

    pub(super) fn on_drag_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.text.len() == 0 {
            return;
        }

        if self.last_layout.is_none() {
            return;
        }

        if !self.focus_handle.is_focused(window) {
            return;
        }

        if !self.selecting {
            return;
        }

        let offset = self.index_for_mouse_position(event.position);
        self.select_to(offset, cx);
    }

    pub(super) fn is_valid_input(&self, new_text: &str, cx: &mut Context<Self>) -> bool {
        if new_text.is_empty() {
            return true;
        }

        if let Some(validate) = &self.validate {
            if !validate(new_text, cx) {
                return false;
            }
        }

        if !self.mask_pattern.is_valid(new_text) {
            return false;
        }

        let Some(pattern) = &self.pattern else {
            return true;
        };

        pattern.is_match(new_text)
    }

    /// Set the mask pattern for formatting the input text.
    ///
    /// The pattern can contain:
    /// - 9: Any digit or dot
    /// - A: Any letter
    /// - *: Any character
    /// - Other characters will be treated as literal mask characters
    ///
    /// Example: "(999)999-999" for phone numbers
    pub fn mask_pattern(mut self, pattern: impl Into<MaskPattern>) -> Self {
        self.mask_pattern = pattern.into();
        if let Some(placeholder) = self.mask_pattern.placeholder() {
            self.placeholder = placeholder.into();
        }
        self
    }

    pub fn set_mask_pattern(
        &mut self,
        pattern: impl Into<MaskPattern>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mask_pattern = pattern.into();
        if let Some(placeholder) = self.mask_pattern.placeholder() {
            self.placeholder = placeholder.into();
        }
        cx.notify();
    }

    pub(super) fn set_input_bounds(&mut self, new_bounds: Bounds<Pixels>, cx: &mut Context<Self>) {
        let wrap_width_changed = self.input_bounds.size.width != new_bounds.size.width;
        self.input_bounds = new_bounds;

        // Update display_map wrap_width if changed.
        if let Some(last_layout) = self.last_layout.as_ref() {
            if wrap_width_changed {
                let wrap_width = if !self.soft_wrap {
                    // None to disable wrapping (will use Pixels::MAX)
                    None
                } else {
                    last_layout.wrap_width
                };

                self.display_map.on_layout_changed(wrap_width, cx);
                self.mode.update_auto_grow(&self.display_map);
                cx.notify();
            }
        }
    }

    pub(super) fn selected_text(&self) -> RopeSlice<'_> {
        let range_utf16 = self.range_to_utf16(&self.selected_range.into());
        let range = self.range_from_utf16(&range_utf16);
        self.text.slice(range)
    }

    /// Return the rendered bounds for a UTF-8 byte range in the current input contents.
    ///
    /// Returns `None` when the requested range is not currently laid out or visible.
    pub fn range_to_bounds(&self, range: &Range<usize>) -> Option<Bounds<Pixels>> {
        let Some(last_layout) = self.last_layout.as_ref() else {
            return None;
        };

        let Some(last_bounds) = self.last_bounds else {
            return None;
        };

        let (_, _, start_pos) = self.line_and_position_for_offset(range.start);
        let (_, _, end_pos) = self.line_and_position_for_offset(range.end);

        let Some(start_pos) = start_pos else {
            return None;
        };
        let Some(end_pos) = end_pos else {
            return None;
        };

        Some(Bounds::from_corners(
            last_bounds.origin + start_pos,
            last_bounds.origin + end_pos + point(px(0.), last_layout.line_height),
        ))
    }

    /// Replace text in range in silent.
    ///
    /// This will not trigger any UI interaction, such as auto-completion.
    pub(crate) fn replace_text_in_range_silent(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.silent_replace_text = true;
        self.replace_text_in_range(range_utf16, new_text, window, cx);
        self.silent_replace_text = false;
    }

    /// Update fold candidates from tree-sitter syntax tree (full extraction).
    /// Used only on initial load or language changes.
    fn update_fold_candidates(&mut self) {
        if !self.mode.is_folding() {
            return;
        }

        let Some(highlighter_rc) = self.mode.highlighter() else {
            return;
        };

        let highlighter = highlighter_rc.borrow();
        let Some(highlighter) = highlighter.as_ref() else {
            return;
        };

        let Some(tree) = highlighter.tree() else {
            return;
        };

        let fold_ranges = crate::input::display_map::extract_fold_ranges(tree);
        self.display_map.set_fold_candidates(fold_ranges);
    }

    /// Incrementally update fold candidates after a text edit.
    /// Only traverses the edited region of the syntax tree instead of the full tree.
    pub(super) fn update_fold_candidates_incremental(&mut self, edit_range: &Range<usize>, new_text: &str) {
        if !self.mode.is_folding() {
            return;
        }

        let Some(highlighter_rc) = self.mode.highlighter() else {
            return;
        };

        let highlighter = highlighter_rc.borrow();
        let Some(highlighter) = highlighter.as_ref() else {
            return;
        };

        let Some(tree) = highlighter.tree() else {
            return;
        };

        // The new byte range in the updated text after the edit
        let new_end = edit_range.start + new_text.len();
        self.display_map.update_fold_candidates_for_edit(
            tree,
            edit_range.start..new_end,
            &self.text,
        );
    }

    /// Spawn a background parse after the synchronous parse timed out.
    ///
    /// Dropping the returned `Task` (stored in `parse_task`) cancels the
    /// parse, which naturally debounces rapid edits.
    #[cfg(not(target_family = "wasm"))]
    pub(super) fn dispatch_background_parse(
        pending: super::mode::PendingBackgroundParse,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let highlighter_rc = pending.highlighter;
        let parse_task_rc = pending.parse_task;
        let language = pending.language;
        let text = pending.text;

        let old_tree = highlighter_rc
            .borrow()
            .as_ref()
            .and_then(|h| h.tree().cloned());

        // Extract injection parse data on the main thread before spawning, so that
        // compute_injection_layers can also run on the background thread.
        let injection_data = highlighter_rc
            .borrow()
            .as_ref()
            .and_then(|h| h.injection_parse_data());

        let text_for_apply = text.clone();
        let task = cx.spawn_in(window, async move |entity, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let Some(config) = LanguageRegistry::singleton().language(&language) else {
                        return None;
                    };

                    let mut parser = tree_sitter::Parser::new();
                    if parser.set_language(&config.language).is_err() {
                        return None;
                    }

                    let new_tree = parser.parse_with_options(
                        &mut |offset, _| {
                            if offset >= text.len() {
                                ""
                            } else {
                                let (chunk, chunk_byte_ix) = text.chunk(offset);
                                &chunk[offset - chunk_byte_ix..]
                            }
                        },
                        old_tree.as_ref(),
                        None,
                    )?;

                    // Compute injection layers in the background to avoid blocking the
                    // main thread with combined-injection parsing (e.g. PHP, HTML+JS/CSS).
                    let injection_layers = if let Some(data) = injection_data {
                        crate::highlighter::SyntaxHighlighter::compute_injection_layers(
                            data, &new_tree, &text,
                        )
                    } else {
                        Default::default()
                    };

                    Some((new_tree, injection_layers))
                })
                .await;

            if let Some((new_tree, injection_layers)) = result {
                if let Some(h) = highlighter_rc.borrow_mut().as_mut() {
                    h.apply_background_tree(new_tree, &text_for_apply, injection_layers);
                }

                // Trigger re-render so the new highlights are displayed.
                _ = entity.update(cx, |_, cx| {
                    cx.notify();
                });
            }
        });

        parse_task_rc.borrow_mut().replace(task);
    }

    #[cfg(target_family = "wasm")]
    pub(super) fn dispatch_background_parse(
        _pending: super::mode::PendingBackgroundParse,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // No-op
    }
}

impl Focusable for InputState {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InputState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self._pending_update {
            let bg = self
                .mode
                .update_highlighter(&(0..0), &self.text, "", false, cx);
            if let Some(bg) = bg {
                Self::dispatch_background_parse(bg, window, cx);
            }

            self.update_fold_candidates();
            self.lsp.update(&self.text, window, cx);
            self._pending_update = false;
        }

        div()
            .id("input-state")
            .flex_1()
            .when(self.mode.is_multi_line(), |this| this.h_full())
            .flex_grow()
            .overflow_x_hidden()
            .child(TextElement::new(cx.entity().clone()).placeholder(self.placeholder.clone()))
            .children(self.diagnostic_popover.clone())
            .children(self.context_menu.as_ref().map(|menu| menu.render()))
            .children(self.hover_popover.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use inazuma::{TestAppContext, VisualTestContext};

    struct InputView {
        input: Entity<InputState>,
        window_handle: inazuma::WindowHandle<Root>,
    }

    /// Helper to create an InputState in a window for testing
    impl InputView {
        pub fn new(cx: &mut TestAppContext) -> Self {
            let mut input: Option<Entity<InputState>> = None;

            let window = cx.update(|cx| {
                cx.open_window(Default::default(), |window, cx| {
                    // Set up the theme first
                    cx.set_global(Theme::default());
                    // Initialize input keybindings
                    super::super::init(cx);

                    input = Some(cx.new(|cx| InputState::new(window, cx).code_editor("sql")));

                    cx.new(|cx| crate::Root::new(input.clone().unwrap(), window, cx))
                })
                .unwrap()
            });

            Self {
                input: input.clone().unwrap(),
                window_handle: window,
            }
        }
    }

    #[inazuma::test]
    fn test_highlighting_preserved_after_fold(cx: &mut TestAppContext) {
        use crate::highlighter::HighlightTheme;
        use crate::input::display_map::FoldRange;

        let input_view = InputView::new(cx);
        let mut cx = VisualTestContext::from_window(input_view.window_handle.into(), cx);
        let input = input_view.input;

        // SQL text: fold the SELECT..WHERE block, verify comments keep highlighting.
        // Lines 0-9: SELECT block (fold range 0..9 hides lines 1-8)
        // Line 10+: comments that must keep highlighting
        let text = "\
SELECT *
FROM users
WHERE id = 1
AND name = 'test'
AND active = true
AND role = 'admin'
AND age > 18
AND status = 'ok'
AND country = 'US'
ORDER BY id

-- Comment 1
-- Comment 2
-- Comment 3";

        cx.update(|window, cx| {
            input.update(cx, |state, cx| {
                state.set_value(text, window, cx);
            });
        });
        cx.run_until_parked();

        // Grab styles for "-- Comment 1" (line 11) before folding
        let theme = HighlightTheme::default_dark();
        let comment_line = 11;
        let comment_start = cx.update(|_, cx| {
            input.read_with(cx, |state, _| state.text.line_start_offset(comment_line))
        });
        let styles_before: Vec<(Range<usize>, inazuma::HighlightStyle)> = cx.update(|_, cx| {
            input.read_with(cx, |state, _| {
                let mode = &state.mode;
                if let crate::input::mode::InputMode::CodeEditor { highlighter, .. } = mode {
                    let h = highlighter.borrow();
                    if let Some(h) = h.as_ref() {
                        let line_end = state.text.line_end_offset(comment_line);
                        return h.styles(&(comment_start..line_end), &theme);
                    }
                }
                vec![]
            })
        });

        // Fold at line 0 with range 0..9 (hides lines 1-8)
        cx.update(|_, cx| {
            input.update(cx, |state, _cx| {
                state
                    .display_map
                    .set_fold_candidates(vec![FoldRange::new(0, 9)]);
                state.display_map.set_folded(0, true);
            });
        });
        cx.run_until_parked();

        // Verify fold is active and lines 1-8 are hidden
        cx.update(|_, cx| {
            input.read_with(cx, |state, _| {
                assert!(state.display_map.is_folded_at(0));
                for line in 1..=8 {
                    assert!(
                        state.display_map.is_buffer_line_hidden(line),
                        "Line {} should be hidden",
                        line
                    );
                }
                assert!(
                    !state.display_map.is_buffer_line_hidden(9),
                    "Line 9 (ORDER BY) should be visible"
                );
            });
        });

        // Get styles for the same comment line after folding
        let styles_after: Vec<(Range<usize>, inazuma::HighlightStyle)> = cx.update(|_, cx| {
            input.read_with(cx, |state, _| {
                let mode = &state.mode;
                if let crate::input::mode::InputMode::CodeEditor { highlighter, .. } = mode {
                    let h = highlighter.borrow();
                    if let Some(h) = h.as_ref() {
                        let line_end = state.text.line_end_offset(comment_line);
                        return h.styles(&(comment_start..line_end), &theme);
                    }
                }
                vec![]
            })
        });

        let colored_before: Vec<_> = styles_before
            .iter()
            .filter(|(_, s)| s.color.is_some())
            .cloned()
            .collect();
        let colored_after: Vec<_> = styles_after
            .iter()
            .filter(|(_, s)| s.color.is_some())
            .cloned()
            .collect();

        assert_eq!(
            colored_before, colored_after,
            "Comment highlighting must be identical before and after folding.\n\
             Before: {:?}\nAfter: {:?}",
            colored_before, colored_after
        );
    }
}
