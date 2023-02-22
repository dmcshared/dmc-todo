use thiserror::Error;

use crate::todo_config::{Group, Todo, TodoConfig};

pub struct PositionHierarchy {
    pub indexes: Vec<usize>, /* indexes except last are group index only (cant have todo in todo). last one is group > todo > todoDone (as drawn on screen) */
}

impl PositionHierarchy {
    pub fn last(&self) -> Result<usize, MoveError> {
        Ok(*self.indexes.last().ok_or(MoveError::NoIndex)?)
    }

    pub fn last_mut(&mut self) -> Result<&mut usize, MoveError> {
        self.indexes.last_mut().ok_or(MoveError::NoIndex)
    }
}

pub struct HierarchyItem<'a> {
    pub item: HierarchyItemEnum<'a>,
    pub depth: usize,
}

pub struct HierarchyItemMut<'a> {
    pub item: HierarchyItemEnumMut<'a>,
    pub depth: usize,
}
pub enum HierarchyItemEnum<'a> {
    Todo(&'a Todo),
    Group(&'a Group),
}
pub enum HierarchyItemEnumMut<'a> {
    Todo(&'a mut Todo),
    Group(&'a mut Group),
}

#[derive(Error, Debug)]
pub enum MoveError {
    #[error("No index items. This should not happen.")]
    NoIndex,
    #[error("The specified group doesn't exist. This should not happen.")]
    GroupNotFound,
    #[error("The specified item doesn't exist. This should not happen.")]
    OutOfBounds,
}

impl PositionHierarchy {
    pub fn new() -> Self {
        Self { indexes: vec![0] }
    }

    pub fn find_item<'a>(&self, context: &'a TodoConfig) -> Result<HierarchyItem<'a>, MoveError> {
        let mut group: &Group = context
            .groups
            .get(self.indexes[0])
            .ok_or(MoveError::GroupNotFound)?;

        let depth = self.indexes.len() - 1;

        if self.indexes.len() == 1 {
            return Ok(HierarchyItem {
                depth,
                item: HierarchyItemEnum::Group(group),
            });
        }

        for i in 1..self.indexes.len() - 1 {
            group = group
                .subgroups
                .get(self.indexes[i])
                .ok_or(MoveError::GroupNotFound)?;
        }

        if *self.indexes.last().ok_or(MoveError::NoIndex)? < group.subgroups.len() {
            // Result is group
            Ok(HierarchyItem {
                depth,
                item: HierarchyItemEnum::Group(
                    group
                        .subgroups
                        .get(*self.indexes.last().ok_or(MoveError::NoIndex)?)
                        .ok_or(MoveError::GroupNotFound)?,
                ),
            })
        } else if *self.indexes.last().ok_or(MoveError::NoIndex)?
            < group.subgroups.len() + group.todos.len()
        {
            Ok(HierarchyItem {
                depth,
                item: HierarchyItemEnum::Todo(
                    group
                        .todos
                        .get(
                            *self.indexes.last().ok_or(MoveError::NoIndex)? - group.subgroups.len(),
                        )
                        .ok_or(MoveError::GroupNotFound)?,
                ),
            })
        } else if *self.indexes.last().ok_or(MoveError::NoIndex)?
            < group.subgroups.len() + group.todos.len() + group.completed.len()
        {
            Ok(HierarchyItem {
                depth,
                item: HierarchyItemEnum::Todo(
                    group
                        .completed
                        .get(
                            *self.indexes.last().ok_or(MoveError::NoIndex)?
                                - group.subgroups.len()
                                - group.todos.len(),
                        )
                        .ok_or(MoveError::GroupNotFound)?,
                ),
            })
        } else {
            Err(MoveError::OutOfBounds)
        }
    }

    pub fn find_item_mut<'a>(
        &self,
        context: &'a mut TodoConfig,
    ) -> Result<HierarchyItemMut<'a>, MoveError> {
        let mut group: &mut Group = context
            .groups
            .get_mut(self.indexes[0])
            .ok_or(MoveError::GroupNotFound)?;

        let depth = self.indexes.len() - 1;

        if self.indexes.len() == 1 {
            return Ok(HierarchyItemMut {
                depth,
                item: HierarchyItemEnumMut::Group(group),
            });
        }

        for i in 1..self.indexes.len() - 1 {
            group = group
                .subgroups
                .get_mut(self.indexes[i])
                .ok_or(MoveError::GroupNotFound)?;
        }

        if *self.indexes.last().ok_or(MoveError::NoIndex)? < group.subgroups.len() {
            // Result is group
            Ok(HierarchyItemMut {
                depth,
                item: HierarchyItemEnumMut::Group(
                    group
                        .subgroups
                        .get_mut(*self.indexes.last().ok_or(MoveError::NoIndex)?)
                        .ok_or(MoveError::GroupNotFound)?,
                ),
            })
        } else if *self.indexes.last().ok_or(MoveError::NoIndex)?
            < group.subgroups.len() + group.todos.len()
        {
            Ok(HierarchyItemMut {
                depth,
                item: HierarchyItemEnumMut::Todo(
                    group
                        .todos
                        .get_mut(
                            *self.indexes.last().ok_or(MoveError::NoIndex)? - group.subgroups.len(),
                        )
                        .ok_or(MoveError::GroupNotFound)?,
                ),
            })
        } else if *self.indexes.last().ok_or(MoveError::NoIndex)?
            < group.subgroups.len() + group.todos.len() + group.completed.len()
        {
            Ok(HierarchyItemMut {
                depth,
                item: HierarchyItemEnumMut::Todo(
                    group
                        .completed
                        .get_mut(
                            *self.indexes.last().ok_or(MoveError::NoIndex)?
                                - group.subgroups.len()
                                - group.todos.len(),
                        )
                        .ok_or(MoveError::GroupNotFound)?,
                ),
            })
        } else {
            Err(MoveError::OutOfBounds)
        }
    }

    // Finds the group ignoring the final index
    pub fn find_group<'a>(&self, context: &'a TodoConfig) -> Result<&'a Group, MoveError> {
        let mut group: &Group = context
            .groups
            .get(self.indexes[0])
            .ok_or(MoveError::GroupNotFound)?;

        if self.indexes.len() == 1 {
            return Ok(group);
        }

        for i in 1..self.indexes.len() - 1 {
            group = group
                .subgroups
                .get(self.indexes[i])
                .ok_or(MoveError::GroupNotFound)?;
        }

        Ok(group)
    }

    // Finds the group ignoring the final index
    pub fn find_group_mut<'a>(
        &self,
        context: &'a mut TodoConfig,
    ) -> Result<&'a mut Group, MoveError> {
        let mut group: &mut Group = context
            .groups
            .get_mut(self.indexes[0])
            .ok_or(MoveError::GroupNotFound)?;

        if self.indexes.len() == 1 {
            return Ok(group);
        }

        for i in 1..self.indexes.len() - 1 {
            group = group
                .subgroups
                .get_mut(self.indexes[i])
                .ok_or(MoveError::GroupNotFound)?;
        }

        Ok(group)
    }

    fn group_size(group: &Group) -> usize {
        if !group.open {
            1
        } else {
            1 + group.subgroups.iter().map(Self::group_size).sum::<usize>()
                + group.todos.len()
                + group.completed.len()
        }
    }

    // moves up on the visible items
    pub fn cursor_up(&mut self, context: &TodoConfig) -> Result<(), MoveError> {
        if *self.indexes.last().ok_or(MoveError::NoIndex)? > 0 {
            self.group_up(context)?;

            if let HierarchyItemEnum::Group(g) = self.find_item(context)?.item {
                if g.open && !g.is_empty() {
                    self.hierarchy_down_no_open(context)?;
                    *self.last_mut()? = g.len() - 1;
                }
            }
        } else {
            self.hierarchy_up(context)?;
        }

        Ok(())
    }

    pub fn cursor_down(&mut self, context: &TodoConfig) -> Result<(), MoveError> {
        if let HierarchyItemEnum::Group(g) = self.find_item(context)?.item {
            if g.open && !g.is_empty() {
                return self.hierarchy_down_no_open(context);
            }
        }

        if self.indexes.len() == 1 {
            // special handling for top-level groups

            self.group_down(context)?;
        }

        let group_count = {
            let group = self.find_group(context)?;
            group.subgroups.len() + group.todos.len() + group.completed.len()
        };

        if *self.indexes.last().ok_or(MoveError::NoIndex)? + 1 >= group_count {
            self.hierarchy_up(context)?;
        }
        self.group_down(context)?;

        Ok(())
    }

    pub fn group_up(&mut self, _context: &TodoConfig) -> Result<(), MoveError> {
        // Moves up in group and doesn't do anything if its at a boundary.

        if *self.indexes.last().ok_or(MoveError::NoIndex)? > 0 {
            *(self.indexes.last_mut().ok_or(MoveError::NoIndex)?) -= 1;
        }

        Ok(())
    }

    pub fn group_down(&mut self, context: &TodoConfig) -> Result<(), MoveError> {
        if self.indexes.len() == 1 {
            if *self.indexes.last().ok_or(MoveError::NoIndex)? < context.groups.len() - 1 {
                *(self.indexes.last_mut().ok_or(MoveError::NoIndex)?) += 1;
            }
            return Ok(());
        }

        let group_count = {
            let group = self.find_group(context)?;
            group.subgroups.len() + group.todos.len() + group.completed.len()
        };

        if *self.indexes.last().ok_or(MoveError::NoIndex)? < group_count - 1 {
            *(self.indexes.last_mut().ok_or(MoveError::NoIndex)?) += 1;
        }

        Ok(())
    }

    pub fn hierarchy_up(&mut self, _context: &TodoConfig) -> Result<(), MoveError> {
        if self.indexes.len() > 1 {
            self.indexes.pop();
        }

        Ok(())
    }

    pub fn hierarchy_down(&mut self, context: &mut TodoConfig) -> Result<(), MoveError> {
        let item = self.find_item_mut(context)?;

        if let HierarchyItemEnumMut::Group(g) = item.item {
            if !g.is_empty() {
                self.indexes.push(0);
                g.open = true;
            }
        }

        Ok(())
    }

    fn hierarchy_down_no_open(&mut self, context: &TodoConfig) -> Result<(), MoveError> {
        let item = self.find_item(context)?;

        if let HierarchyItemEnum::Group(g) = item.item {
            if !g.is_empty() {
                self.indexes.push(0);
            }
        }

        Ok(())
    }

    pub fn vert_pos(&self, context: &TodoConfig) -> Result<usize, MoveError> {
        // Finds the vertical position of the cursor in the context. 0 is top level group.
        let mut total = 0;

        for i in 0..*self.indexes.first().ok_or(MoveError::NoIndex)? {
            total += Self::group_size(context.groups.get(i).ok_or(MoveError::GroupNotFound)?);
        }

        let mut current_group = context
            .groups
            .get(*self.indexes.first().ok_or(MoveError::NoIndex)?)
            .ok_or(MoveError::GroupNotFound)?;

        for i in 1..self.indexes.len() {
            total += 1; // Header

            if self.indexes[i] < current_group.subgroups.len() {
                // in group
                for i in 0..self.indexes[i] {
                    total += Self::group_size(
                        current_group
                            .subgroups
                            .get(i)
                            .ok_or(MoveError::GroupNotFound)?,
                    );
                }
                current_group = current_group
                    .subgroups
                    .get(self.indexes[i])
                    .ok_or(MoveError::GroupNotFound)?;
            } else {
                // in notes
                total += current_group
                    .subgroups
                    .iter()
                    .map(Self::group_size)
                    .sum::<usize>();
                total += self.indexes[i] - current_group.subgroups.len();
            }
        }

        Ok(total)
    }

    pub fn vert_offset(&self, context: &TodoConfig) -> Result<usize, MoveError> {
        // keep cursor within 50% middle

        let height = crossterm::terminal::size().unwrap_or((20, 10)).1;

        let cursor = self.vert_pos(context)?;

        Ok((cursor >> 2) * height as usize)
    }

    pub fn vert_pos_offset(&self, context: &TodoConfig) -> Result<usize, MoveError> {
        Ok(self.vert_pos(context)? - self.vert_offset(context)?)
    }
}

impl Default for PositionHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

pub enum Cursor {
    Hierarchy(PositionHierarchy),
    // Flat(FlatHierarchy),
}
