use ratatui::{Frame, layout::Rect};
use alloc::vec::Vec;
use crate::{
    events::{InputEvent, TaskSenders},
};

pub mod main_menu;
pub mod search_and_destroy;
pub mod battlefield;
pub mod the_finals;

/// Available views/screens in the application
#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    MainMenu,
    SearchAndDestroy,
    Battlefield,
    TheFinals,
}

/// Navigation actions that can be requested by views
#[derive(Debug, Clone)]
pub enum NavigationAction {
    GoTo(ViewType),
    Back,
    Exit,
}

/// Common trait for all views
pub trait View {
    /// Handle input events and optionally return navigation actions
    fn handle_input(&mut self, event: InputEvent, task_senders: &TaskSenders) -> Option<NavigationAction>;
    
    /// Render the view
    fn render(&self, frame: &mut Frame, area: Rect);
    
    /// Called when entering this view
    fn on_enter(&mut self, _task_senders: &TaskSenders) {}
    
    /// Called when leaving this view
    fn on_exit(&mut self, _task_senders: &TaskSenders) {}
    
    /// Get the view type
    fn view_type(&self) -> ViewType;
}

/// Router manages navigation between views
pub struct Router {
    current_view: ViewType,
    view_stack: Vec<ViewType>, // History stack for back navigation
    main_menu: main_menu::MainMenuView,
    search_and_destroy: search_and_destroy::SearchAndDestroyView,
    battlefield: battlefield::BattlefieldView,
    the_finals: the_finals::TheFinalsView,
}

impl Router {
    pub fn new(task_senders: &TaskSenders) -> Self {
        let mut router = Self {
            current_view: ViewType::MainMenu,
            view_stack: Vec::new(),
            main_menu: main_menu::MainMenuView::new(),
            search_and_destroy: search_and_destroy::SearchAndDestroyView::new(),
            battlefield: battlefield::BattlefieldView::new(),
            the_finals: the_finals::TheFinalsView::new(),
        };
        
        // Initialize the main menu view
        router.get_current_view_mut().on_enter(task_senders);
        router
    }
    
    pub fn current_view(&self) -> ViewType {
        self.current_view.clone()
    }
    
    pub fn handle_input(&mut self, event: InputEvent, task_senders: &TaskSenders) -> Option<NavigationAction> {
        let action = self.get_current_view_mut().handle_input(event, task_senders);
        
        if let Some(ref nav_action) = action {
            self.handle_navigation(nav_action.clone(), task_senders);
        }
        
        action
    }
    
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        self.get_current_view().render(frame, area);
    }
    
    fn handle_navigation(&mut self, action: NavigationAction, task_senders: &TaskSenders) {
        match action {
            NavigationAction::GoTo(new_view) => {
                if new_view != self.current_view {
                    // Exit current view
                    self.get_current_view_mut().on_exit(task_senders);
                    
                    // Push current view to stack for back navigation
                    self.view_stack.push(self.current_view.clone());
                    
                    // Switch to new view
                    self.current_view = new_view;
                    self.get_current_view_mut().on_enter(task_senders);
                }
            },
            NavigationAction::Back => {
                if let Some(previous_view) = self.view_stack.pop() {
                    // Exit current view
                    self.get_current_view_mut().on_exit(task_senders);
                    
                    // Go back to previous view
                    self.current_view = previous_view;
                    self.get_current_view_mut().on_enter(task_senders);
                }
            },
            NavigationAction::Exit => {
                // Handle application exit if needed
                // For now, just go back to main menu
                self.navigate_to_main_menu(task_senders);
            }
        }
    }
    
    fn navigate_to_main_menu(&mut self, task_senders: &TaskSenders) {
        if self.current_view != ViewType::MainMenu {
            self.get_current_view_mut().on_exit(task_senders);
            self.current_view = ViewType::MainMenu;
            self.view_stack.clear(); // Clear history when going to main menu
            self.get_current_view_mut().on_enter(task_senders);
        }
    }
    
    fn get_current_view(&self) -> &dyn View {
        match self.current_view {
            ViewType::MainMenu => &self.main_menu,
            ViewType::SearchAndDestroy => &self.search_and_destroy,
            ViewType::Battlefield => &self.battlefield,
            ViewType::TheFinals => &self.the_finals,
        }
    }
    
    fn get_current_view_mut(&mut self) -> &mut dyn View {
        match self.current_view {
            ViewType::MainMenu => &mut self.main_menu,
            ViewType::SearchAndDestroy => &mut self.search_and_destroy,
            ViewType::Battlefield => &mut self.battlefield,
            ViewType::TheFinals => &mut self.the_finals,
        }
    }
}
