//! Reusable RPlayer controls: icon buttons, sidebar rows, search field,
//! the mint seek bar and custom volume control.

use egui::{
    Align2, Button, Color32, CornerRadius, CursorIcon, FontId, Frame, Margin, Pos2, Rect, Response,
    RichText, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2, WidgetInfo, WidgetType,
};

use crate::theme;
use crate::ui::icons::{self, Icon};

mod buttons;
mod nav;
mod inputs;
mod chips;
mod toggles;
mod shapes;

pub(crate) use buttons::*;
pub(crate) use nav::*;
pub(crate) use inputs::*;
pub(crate) use chips::*;
pub(crate) use toggles::*;
pub(crate) use shapes::*;
