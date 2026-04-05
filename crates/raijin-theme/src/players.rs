use inazuma::Oklch;

/// Colors for a single player/collaborator in a multiplayer editing session.
#[derive(Clone, Debug)]
pub struct PlayerColor {
    /// The cursor color for this player.
    pub cursor: Oklch,
    /// The background color for this player's highlights.
    pub background: Oklch,
    /// The selection color for this player.
    pub selection: Oklch,
}

/// A set of player colors for distinguishing multiple collaborators.
#[derive(Clone, Debug)]
pub struct PlayerColors(pub Vec<PlayerColor>);

impl PlayerColors {
    /// Returns the player color at the given index, wrapping around if needed.
    pub fn color_for_participant(&self, index: usize) -> &PlayerColor {
        &self.0[index % self.0.len()]
    }

    /// Returns the number of player colors available.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if there are no player colors.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
