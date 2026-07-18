use adw::{glib::property::PropertySet, prelude::*, subclass::prelude::*};
use gtk::{
    Ordering,
    gio::Icon,
    glib::{self},
};
use log::trace;

use super::graph::ResGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeterType {
    NoMeter,
    Graph,
    ProgressBar,
}

mod imp {
    use std::cell::{Cell, RefCell};

    use crate::ui::widgets::graph::ResGraph;

    use super::*;

    use gtk::{
        CompositeTemplate,
        gio::{Icon, ThemedIcon},
        glib::{ParamSpec, Properties, Value},
    };

    #[derive(CompositeTemplate, Properties)]
    #[template(resource = "/org/gnome/Resources/ui/widgets/stack_sidebar_item.ui")]
    #[properties(wrapper_type = super::ResStackSidebarItem)]
    pub struct ResStackSidebarItem {
        #[template_child]
        pub image: TemplateChild<gtk::Image>,
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
        #[template_child]
        pub detail_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub meter_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub no_meter_page: TemplateChild<gtk::Box>,
        #[template_child]
        pub graph_page: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub graph: TemplateChild<ResGraph>,
        #[template_child]
        pub graph_usage_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub progress_page: TemplateChild<gtk::Box>,
        #[template_child]
        pub usage_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_label_bar: TemplateChild<gtk::Label>,
        #[template_child]
        pub progress_bar: TemplateChild<gtk::ProgressBar>,

        #[property(get = Self::name, set = Self::set_name, type = glib::GString)]
        name: Cell<glib::GString>,
        #[property(get = Self::detail, set = Self::set_detail, type = glib::GString)]
        detail: Cell<glib::GString>,
        #[property(get = Self::subtitle, set = Self::set_subtitle, type = glib::GString)]
        subtitle: Cell<glib::GString>,
        #[property(get = Self::icon, set = Self::set_icon, type = Icon)]
        icon: RefCell<Icon>,
        #[property(get, set = Self::set_usage)]
        usage: Cell<f64>,
        #[property(get = Self::graph_usage_label_text, set = Self::set_graph_usage_label_text, type = glib::GString)]
        graph_usage_label_text: RefCell<glib::GString>,
        #[property(get = Self::tab_id, set = Self::set_tab_id, type = glib::GString)]
        tab_id: Cell<glib::GString>,

        pub primary_ord: Cell<u32>,
        pub secondary_ord: Cell<u32>,
    }

    impl ResStackSidebarItem {
        pub fn name(&self) -> glib::GString {
            let name = self.name.take();
            self.name.set(name.clone());
            name
        }

        pub fn set_name(&self, name: &str) {
            let current_name = self.name.take();
            if current_name.as_str() == name {
                self.name.set(current_name);
                return;
            }
            self.name.set(glib::GString::from(name));
            self.label.set_label(name);
        }

        pub fn subtitle(&self) -> glib::GString {
            let subtitle = self.subtitle.take();
            self.subtitle.set(subtitle.clone());
            subtitle
        }

        pub fn set_subtitle(&self, usage_string: &str) {
            let current_usage_string = self.subtitle.take();
            if current_usage_string.as_str() == usage_string {
                self.subtitle.set(current_usage_string);
                return;
            }
            self.usage_label.set_label(usage_string);
        }

        pub fn detail(&self) -> glib::GString {
            let detail = self.detail.take();
            self.detail.set(detail.clone());
            detail
        }

        pub fn set_detail(&self, detail: &str) {
            let current_detail = self.detail.take();
            if current_detail.as_str() == detail {
                self.detail.set(current_detail);
                return;
            }
            self.detail_label.set_label(detail);
        }

        pub fn icon(&self) -> Icon {
            let icon = self
                .icon
                .replace_with(|_| ThemedIcon::new("generic-process").into());
            self.icon.set(icon.clone());
            icon
        }

        pub fn set_icon(&self, icon: &Icon) {
            self.image.set_from_gicon(icon);
            self.icon.set(icon.clone());
        }

        pub fn graph_usage_label_text(&self) -> glib::GString {
            self.graph_usage_label_text.borrow().clone()
        }

        pub fn set_graph_usage_label_text(&self, text: &str) {
            self.graph_usage_label_text.set(glib::GString::from(text));
            self.graph_usage_label.set_label(text);
        }

        pub fn set_usage(&self, usage: f64) {
            self.usage.set(usage);

            let mut highest_value = self.graph.get_highest_value();
            if highest_value < 1.0 {
                highest_value = 1.0;
            }

            self.progress_bar.set_fraction(usage / highest_value);

            self.graph.push_data_point(usage);
        }

        gstring_getter_setter!(tab_id);
    }

    impl Default for ResStackSidebarItem {
        fn default() -> Self {
            Self {
                image: Default::default(),
                label: Default::default(),
                meter_stack: Default::default(),
                no_meter_page: Default::default(),
                graph_page: Default::default(),
                graph: Default::default(),
                graph_usage_label: Default::default(),
                progress_page: Default::default(),
                usage_label: Default::default(),
                usage_label_bar: Default::default(),
                progress_bar: Default::default(),
                detail_label: Default::default(),
                name: Default::default(),
                detail: Default::default(),
                subtitle: Default::default(),
                icon: RefCell::new(ThemedIcon::new("generic-process").into()),
                usage: Default::default(),
                graph_usage_label_text: RefCell::new(glib::GString::new()),
                tab_id: Default::default(),
                primary_ord: Default::default(),
                secondary_ord: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ResStackSidebarItem {
        const NAME: &'static str = "ResStackSidebarItem";
        type Type = super::ResStackSidebarItem;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ResStackSidebarItem {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }

    impl WidgetImpl for ResStackSidebarItem {}

    impl BinImpl for ResStackSidebarItem {}
}

glib::wrapper! {
    pub struct ResStackSidebarItem(ObjectSubclass<imp::ResStackSidebarItem>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Buildable, gtk::ConstraintTarget, gtk::Accessible;
}

impl ResStackSidebarItem {
    pub fn new(
        name: String,
        icon: Icon,
        detail: Option<String>,
        usage_string: String,
        locked_max_y: bool,
        tab_id: String,
        primary_ord: u32,
        secondary_ord: u32,
    ) -> Self {
        trace!("Creating ResStackSidebarItem GObject…");

        let detail = detail.unwrap_or_default();
        let this: Self = glib::Object::builder()
            .property("name", name)
            .property("icon", icon)
            .property("detail", &detail)
            .property("subtitle", usage_string)
            .build();

        this.imp()
            .graph
            .set_locked_max_y(locked_max_y.then_some(1.0));
        this.imp().graph.set_height_request(64);

        this.imp().detail_label.set_visible(!detail.is_empty());

        this.imp().set_tab_id(&tab_id);

        this.imp().primary_ord.set(primary_ord);
        this.imp().secondary_ord.set(secondary_ord);

        this
    }

    pub fn graph(&self) -> ResGraph {
        self.imp().graph.get()
    }

    pub fn set_meter_type(&self, meter_type: MeterType) {
        let imp = self.imp();
        let no_meter = imp.no_meter_page.get();
        let graph = imp.graph_page.get();
        let progress = imp.progress_page.get();

        no_meter.set_visible(false);
        graph.set_visible(false);
        progress.set_visible(false);

        match meter_type {
            MeterType::NoMeter => {
                no_meter.set_visible(true);
                self.imp().meter_stack.set_visible_child(&no_meter);
            }
            MeterType::Graph => {
                graph.set_visible(true);
                self.imp().meter_stack.set_visible_child(&graph);
            }
            MeterType::ProgressBar => {
                progress.set_visible(true);
                self.imp().meter_stack.set_visible_child(&progress);
            }
        }
    }

    pub fn graph_usage_label(&self) -> gtk::Label {
        self.imp().graph_usage_label.get()
    }

    pub fn set_detail_label_visible(&self, visible: bool) {
        self.imp().detail_label.set_visible(visible);
    }

    pub fn primary_ord(&self) -> u32 {
        self.imp().primary_ord.clone().take()
    }

    pub fn secondary_ord(&self) -> u32 {
        self.imp().secondary_ord.clone().take()
    }

    pub fn ord(&self, other: &Self) -> Ordering {
        if self.primary_ord() > other.primary_ord() {
            Ordering::Larger
        } else if self.primary_ord() < other.primary_ord() {
            Ordering::Smaller
        } else if self.secondary_ord() > other.secondary_ord() {
            Ordering::Larger
        } else if self.secondary_ord() < other.secondary_ord() {
            Ordering::Smaller
        } else {
            Ordering::Equal
        }
    }
}
