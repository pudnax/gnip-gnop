use crate::math::{Vec2, Vec4};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GameState {
    MainMenu,
    Serving,
    Playing,
    GameOver,
    Quiting,
    Base,
}

pub struct State {
    pub ball: Ball,
    pub player1: Player,
    pub player2: Player,
    pub title_text: Text,
    pub play_button: Text,
    pub quit_button: Text,
    pub player1_score: Text,
    pub player2_score: Text,
    pub win_text: Text,
    pub game_state: GameState,
    pub prev_state: GameState,
}

pub struct Ball {
    pub position: Vec2,
    pub velocity: Vec2,
    pub radius: f32,
    pub visible: bool,
}

#[derive(Debug)]
pub struct Player {
    pub position: Vec2,
    pub size: Vec2,
    pub score: u32,
    pub visible: bool,
}

impl Player {
    pub fn contains(&self, ball: &Ball) -> bool {
        let radii = self.size * 0.5;
        let min = self.position - radii;
        let max = self.position + radii;

        let b_radii = Vec2 {
            x: ball.radius,
            y: ball.radius,
        };
        let b_min = ball.position - b_radii;
        let b_max = ball.position + b_radii;

        min.x < b_max.x && max.x > b_min.x && min.y < b_max.y && max.y > b_min.y
    }
}

pub const UNBOUNDED_F32: f32 = std::f32::INFINITY;

#[derive(Debug)]
pub struct Text {
    pub position: Vec2,
    pub bounds: Vec2,
    pub color: Vec4,
    pub text: String,
    pub size: f32,
    pub visible: bool,
    pub focused: bool,
    pub centered: bool,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            position: (0.0, 0.0).into(),
            bounds: (UNBOUNDED_F32, UNBOUNDED_F32).into(),
            color: (1.0, 1.0, 1.0, 1.0).into(),
            text: String::new(),
            size: 16.0,
            visible: false,
            focused: false,
            centered: false,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Event {
    ButtonPressed,
    FocusChanged,
    BallBounce(Vec2),
    Score(u32),
}
