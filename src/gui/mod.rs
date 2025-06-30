use std::error::Error;
use eframe::{App, Frame};
use eframe::epaint::text::TextWrapMode;
use egui::{Button, CentralPanel, Color32, Context, FontData, Id, Label, ScrollArea, Sense, Ui, Visuals};
use crate::data::{FilterMode, HttpTcpData};

pub struct ProxyView {
    data: Vec<HttpTcpData>,
    current_item: Option<usize>,
    working: bool,
    filter_mode: FilterMode,
}

impl ProxyView {
    pub fn new(ctx: &eframe::CreationContext) -> Result<Box<dyn App>, Box<dyn Error + Send + Sync + 'static>> {
        //修改默认字体，确保支持中文
        let mut fonts = egui::FontDefinitions::default();
        let font_bytes = include_bytes!("../../res/font/simfang.ttf");
        let font_data = FontData::from_static(font_bytes);
        fonts.font_data.insert("my_font".to_owned(), std::sync::Arc::new(font_data));
        fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "my_font".to_owned());
        fonts.families.entry(egui::FontFamily::Monospace).or_default().push("my_font".to_owned());
        ctx.egui_ctx.set_fonts(fonts);
        //修改为亮模式
        ctx.egui_ctx.set_visuals(Visuals::light());
        //安装图片加载器
        egui_extras::install_image_loaders(&ctx.egui_ctx);
        Ok(Box::new(ProxyView {
            data: vec![HttpTcpData::new()],
            current_item: None,
            working: false,
            filter_mode: FilterMode::None,
        }))
    }

    fn show_root_top(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // ui.painter().rect_filled(ui.max_rect(), 0.0, Color32::BLUE);
            // ui.set_height(50.0);
            let btn = Button::image_and_text("", if self.working { "停止" } else { "启动" });
            ui.add(btn).clicked().then(|| self.working = !self.working);
            let btn = Button::image_and_text("", "保存");
            ui.add(btn).clicked().then(|| {});
            let btn = Button::image_and_text("", "导出");
            ui.add(btn).clicked().then(|| {});
            for mode in FilterMode::modes() {
                ui.selectable_label(self.filter_mode == mode, mode.to_string()).clicked().then(|| self.filter_mode = mode);
            }
        });
    }

    fn show_item(&mut self, index: usize, ui: &mut Ui) {
        let datum = &self.data[index];
        let rect = ui.max_rect();
        ui.vertical(|ui| {
            //这里的样式我们后面再更换
            if let Some(current_item) = self.current_item {
                if current_item == index {
                    ui.painter().rect_filled(rect, 0.2, Color32::LIGHT_BLUE);
                }
            }
            let resp = ui.interact(rect, Id::from(format!("item_{}", index)), Sense::click_and_drag());
            if resp.hovered() {
                ui.painter().rect_filled(rect, 0.2, Color32::LIGHT_YELLOW);
            }
            if resp.clicked() {
                self.current_item = Some(index);
            }
            //保证不自动换行
            let url = Label::new("https://docs.rs/eframe/latest/eframe/").wrap_mode(TextWrapMode::Extend).truncate();
            ui.add(url);
            ui.horizontal(|ui| {
                ui.label(index.to_string());
                ui.label(200.to_string());
                ui.label("文档");
                ui.label("08:00");
                ui.label("1.6 Kb");
            });
        });
    }

    fn shou_root_middle_left(&mut self, ui: &mut Ui) {
        /*
          -------------------------------------
          |  URL                              |
          -------------------------------------
          | 编 号 | 状态码 | 类型 | 时间 | 总大小 |
          ------------------------------------
         */
        ui.vertical(|ui| {
            let area = ScrollArea::vertical().auto_shrink([false; 2]).stick_to_bottom(true);
            area.show_rows(ui, 50.0, self.data.len(), |ui, rows| {
                for row in rows { self.show_item(row, ui); }
            });
        });
    }
}

impl App for ProxyView {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            self.show_root_top(ui);
            self.shou_root_middle_left(ui);
        });
    }
}