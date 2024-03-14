use crate::{compiler::Error as CompilerError, vm::Error as VMError};

#[derive(Debug, Clone)]
enum ApplicationError {
    CompileError(CompilerError),
    VirtualmachineError(VMError),
}

impl std::fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ApplicationError {}

impl From<CompilerError> for ApplicationError {
    fn from(value: CompilerError) -> Self {
        Self::CompileError(value)
    }
}

impl From<VMError> for ApplicationError {
    fn from(value: VMError) -> Self {
        Self::VirtualmachineError(value)
    }
}

#[cfg(not(feature = "gui"))]
mod terminal {
    use super::ApplicationError;
    use std::io::{self, Write};

    use crate::{
        compiler::{Compile, Compiler},
        lexer::Lexer,
        vm::VirtualMachine,
    };

    fn run_file(src: &[u8]) -> Result<f64, ApplicationError> {
        let mut lexer = Lexer::new(src);
        let mut compiler = Compiler::default();
        compiler.compile(&mut lexer)?;
        let mut vm = VirtualMachine::default();
        vm.interpret(compiler.opcodes()).map_err(|e| e.into())
    }

    fn run_repl() -> ! {
        let mut input = String::new();
        let mut compiler = Compiler::default();
        let mut vm = VirtualMachine::default();
        loop {
            print!(">> ");
            io::stdout().flush().unwrap();
            input.clear();
            if io::stdin().read_line(&mut input).is_err() {
                continue;
            }
            if input == "\n" || input == "\r\n" {
                continue;
            }
            let bytes = input.as_bytes();
            let mut lexer = Lexer::new(bytes);
            if let Err(e) = compiler.compile(&mut lexer) {
                eprintln!("Compiler error: {}", e);
                compiler.reset();
                continue;
            }
            let ans = match vm.interpret(compiler.opcodes()) {
                Ok(value) => {
                    println!("$ {}", value);
                    Some(value)
                }
                Err(e) => {
                    eprintln!("Virtual machine error: {}", e);
                    None
                }
            };
            vm.reset(ans);
            compiler.reset();
        }
    }

    pub fn run() -> std::process::ExitCode {
        match std::env::args().nth(1) {
            Some(src_path) => {
                let src =
                    std::fs::read(src_path).unwrap_or_else(|e| panic!("Cannot read file {}", e));
                match run_file(&src) {
                    Ok(res) => {
                        println!("Result of computation: {}", res);
                        std::process::ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::ExitCode::FAILURE
                    }
                }
            }
            None => run_repl(),
        }
    }
}

#[cfg(not(feature = "gui"))]
pub use terminal::*;

#[cfg(feature = "gui")]
mod gui {
    use eframe::egui;
    use std::collections::VecDeque;

    use crate::{
        compiler::{Compile, Compiler},
        lexer::Lexer,
        vm::VirtualMachine,
    };

    use super::ApplicationError;

    const MAX_HISTORY_LEN: usize = 10;

    struct App {
        expressions: VecDeque<(String, Option<f64>)>,
        expression_index: usize,
        result: String,
        compiler: Compiler,
        vm: VirtualMachine,
    }

    impl Default for App {
        fn default() -> Self {
            let mut expressions = VecDeque::new();
            expressions.push_back(("".to_owned(), None));
            Self {
                expressions,
                expression_index: 0,
                result: "".to_owned(),
                compiler: Compiler::default(),
                vm: VirtualMachine::default(),
            }
        }
    }

    impl eframe::App for App {
        fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Calculator");
                ui.label(&self.expressions[self.index()].0);
                ui.label(&self.result);
                self.buttons(ui);
            });
        }
    }

    impl App {
        fn index(&self) -> usize {
            self.expressions.len() - self.expression_index - 1
        }

        fn is_current_expression(&self) -> bool {
            self.expression_index == 0
        }

        fn solve(&mut self) {
            let (s, ans) = &self.expressions[self.index()];
            let mut lexer = Lexer::new(s.as_bytes());
            self.vm.reset(*ans);
            let res = self
                .compiler
                .compile(&mut lexer)
                .map_err(ApplicationError::from)
                .and_then(|_| {
                    self.vm
                        .interpret(self.compiler.opcodes())
                        .map_err(|e| e.into())
                });
            match res {
                Ok(r) => {
                    self.result = format!("{:+e}", r);
                    if self.is_current_expression() {
                        self.expressions.push_back(("".to_owned(), Some(r)));
                    }
                    if self.expressions.len() >= MAX_HISTORY_LEN {
                        self.expressions.pop_front();
                    }
                    self.compiler.reset();
                }
                Err(e) => {
                    self.result = e.to_string();
                    if let Some((s, _)) = self.expressions.back_mut() {
                        s.clear();
                    }
                    self.compiler.reset();
                }
            };
        }

        fn draw_number_row(&mut self, ui: &mut egui::Ui, nums: [&'static str; 3]) {
            for num in nums {
                if ui.add(single_char_btn(num)).clicked() && self.is_current_expression() {
                    if let Some((s, _)) = self.expressions.back_mut() {
                        s.push_str(num)
                    }
                }
            }
        }

        fn draw_function(&mut self, ui: &mut egui::Ui, fname: &str) {
            ui.scope(|ui| {
                ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);
                if ui
                    .add(large_btn(fname).fill(egui::Color32::from_rgb(51, 66, 255)))
                    .clicked()
                    && self.is_current_expression()
                {
                    if let Some((s, _)) = self.expressions.back_mut() {
                        s.push_str(fname);
                        s.push('(');
                    }
                }
            });
        }

        fn buttons(&mut self, ui: &mut egui::Ui) {
            ui.style_mut().spacing.item_spacing = egui::Vec2::new(1.0, 1.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.add(single_char_btn("(")).clicked() && self.is_current_expression() {
                        if let Some((s, _)) = self.expressions.back_mut() {
                            s.push('(');
                        }
                    }
                    if ui.add(single_char_btn(")")).clicked() && self.is_current_expression() {
                        if let Some((s, _)) = self.expressions.back_mut() {
                            s.push(')');
                        }
                    }
                    ui.scope(|ui| {
                        ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
                        if ui
                            .add(large_btn("=").fill(egui::Color32::from_rgb(255, 165, 51)))
                            .clicked()
                        {
                            self.solve();
                        }
                    });
                    self.draw_function(ui, "cos");
                    self.draw_function(ui, "sqrt");
                });
                ui.horizontal(|ui| {
                    self.draw_number_row(ui, ["1", "2", "3"]);
                    if ui.add(single_char_btn("+")).clicked() && self.is_current_expression() {
                        if let Some((s, _)) = self.expressions.back_mut() {
                            s.push('+');
                        }
                    }
                    self.draw_function(ui, "sin");
                    self.draw_function(ui, "log");
                });

                ui.horizontal(|ui| {
                    self.draw_number_row(ui, ["4", "5", "6"]);
                    if ui.add(single_char_btn("-")).clicked() && self.is_current_expression() {
                        if let Some((s, _)) = self.expressions.back_mut() {
                            s.push('-');
                        }
                    }
                    self.draw_function(ui, "pow");
                    if ui.add(large_btn("10^x")).clicked() && self.is_current_expression() {
                        if let Some((s, _)) = self.expressions.back_mut() {
                            s.push('e');
                        }
                    }
                });
                ui.horizontal(|ui| {
                    self.draw_number_row(ui, ["7", "8", "9"]);
                    if ui.add(single_char_btn("*")).clicked() && self.is_current_expression() {
                        if let Some((s, _)) = self.expressions.back_mut() {
                            s.push('*');
                        }
                    }
                    if ui.add(large_btn("ans")).clicked() && self.is_current_expression() {
                        if let Some((s, _)) = self.expressions.back_mut() {
                            s.push_str("ans");
                        }
                    }
                    if ui.add(large_btn("Del")).clicked() && self.is_current_expression() {
                        if let Some((s, _)) = self.expressions.back_mut() {
                            s.pop();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.add(single_char_btn("0")).clicked() && self.is_current_expression() {
                        if let Some((s, _)) = self.expressions.back_mut() {
                            s.push('0');
                        }
                    }
                    if ui.add(single_char_btn(".")).clicked() && self.is_current_expression() {
                        if let Some((s, _)) = self.expressions.back_mut() {
                            s.push('.');
                        }
                    }
                    if ui.add(single_char_btn(",")).clicked() && self.is_current_expression() {
                        if let Some((s, _)) = self.expressions.back_mut() {
                            s.push(',');
                        }
                    }
                    if ui.add(single_char_btn("/")).clicked() && self.is_current_expression() {
                        if let Some((s, _)) = self.expressions.back_mut() {
                            s.push('/');
                        }
                    }
                    if ui.add(large_btn("prev")).clicked() {
                        let idx = self.expression_index + 1;
                        if self.expressions.len() > idx {
                            self.expression_index = idx;
                        }
                    }
                    if ui.add(large_btn("next")).clicked() && !self.is_current_expression() {
                        self.expression_index -= 1;
                    }
                });
            });
        }
    }

    fn single_char_btn(n: &str) -> egui::Button {
        egui::Button::new(n).min_size(egui::Vec2::new(BTN_WIDTH, BTN_HEGHT))
    }

    fn large_btn(n: &str) -> egui::Button {
        egui::Button::new(n).min_size(egui::Vec2::new(BTN_LARGE_WIDTH, BTN_HEGHT))
    }

    const BTN_HEGHT: f32 = 20.0;
    const BTN_WIDTH: f32 = 20.0;
    const BTN_LARGE_WIDTH: f32 = 2. * BTN_WIDTH;
    const W_WIDTH: f32 = 8. * BTN_WIDTH + 20.;
    const W_HEIGHT: f32 = 200.0;

    pub fn run() -> std::process::ExitCode {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([W_WIDTH, W_HEIGHT]),
            ..Default::default()
        };
        match eframe::run_native("Calculator", options, Box::new(|_| Box::<App>::default())) {
            Ok(_) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("Cannot run application: {}", e);
                std::process::ExitCode::FAILURE
            }
        }
    }
}

#[cfg(feature = "gui")]
pub use gui::*;
