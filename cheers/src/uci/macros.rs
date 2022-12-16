macro_rules! uci_options {
    [$($opt_name:ident ( $($opt_def:tt)* )),* $(,)?] => {
        pub enum UciOption {
            $($opt_name ( extract_value_type!( $($opt_def)*) )),*
        }

        impl UciOption {
            pub fn parse(name: &str, value: &str) -> Result<Self, UciParseError> {
                $(
                    if name.to_lowercase() == stringify!($opt_name).to_lowercase() {
                        match UCI_OPTIONS.$opt_name.1.validate(value) {
                            Ok(val) => return Ok(UciOption::$opt_name(val)),
                            Err(e) => return Err(UciParseError::Other(format!(
                                        concat!("Invalid value for ",
                                                stringify!($opt_name),
                                                " in UCI setoption command: {}\n\t{}"
                                        ), value, e))
                                )
                        }
                    }
                )*

                unreachable!()
            }
        }
        #[allow(non_snake_case)]
        struct UciOptions {
            $($opt_name : (&'static str, extract_opt_type_concrete!( $($opt_def)* ))),*
        }

        const UCI_OPTIONS: UciOptions = UciOptions {
            $(
                $opt_name : (stringify!($opt_name), extract_opt_initializers!($($opt_def)*))
            ),*
        };

        pub fn print_uci_options() {
            $(
                println!("option name {} {}",
                         UCI_OPTIONS.$opt_name.0,
                         UCI_OPTIONS.$opt_name.1.details());
            )*
        }
    };
}

macro_rules! extract_value_type {
    ($opt_type:ident $opt_init:tt) => {
        <$opt_type as ValidateOption>::Output
    };

    ($opt_type:ident < $value_type:ty > $opt_init:tt) => {
        $value_type
    };
}

macro_rules! extract_opt_type_concrete {
    ($opt_type:ident $opt_init:tt) => {
        $opt_type
    };

    ($opt_type:ident < $value_type:ty > $opt_init:tt) => {
        $opt_type<$value_type>
    };
}

macro_rules! extract_opt_initializers {
    ($opt_type:ident $opt_init:tt) => {
        $opt_type $opt_init
    };

    ($opt_type:ident < $value_type:ty > $opt_init:tt) => {
        $opt_type $opt_init
    };
}
