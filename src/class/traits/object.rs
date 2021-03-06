use std::convert::From;

use binding::class;
use binding::global::ValueType;
use binding::util as binding_util;
use result::{Error, Result};
use types::{Callback, Value};
use util;

use {AnyObject, Class, VerifiedObject};

/// `Object`
///
/// Trait consists methods of Ruby `Object` class. Every struct like `Array`, `Hash` etc implements
/// this trait.
///
/// `class!` macro automatically implements this trait for custom classes.
pub trait Object: From<Value> {
    /// Returns internal `value` of current object.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruru::types::Value;
    /// use ruru::Object;
    ///
    /// struct Array {
    ///     value: Value
    /// }
    ///
    /// impl From<Value> for Array {
    ///     fn from(value: Value) -> Self {
    ///         Array {
    ///             value: value
    ///         }
    ///     }
    /// }
    ///
    /// impl Object for Array {
    ///     fn value(&self) -> Value {
    ///         self.value
    ///     }
    /// }
    /// ```
    #[inline]
    fn value(&self) -> Value;

    /// Returns a class of current object.
    ///
    /// # Examples
    /// ```
    /// use ruru::{Array, Object, VM};
    /// # VM::init();
    ///
    /// assert_eq!(Array::new().class(), Array::new().class());
    /// ```
    fn class(&self) -> Class {
        let class = class::object_class(self.value());

        Class::from(class)
    }

    /// Returns a singleton class of current object.
    ///
    /// # Examples
    ///
    /// ### Getting singleton class
    ///
    /// ```
    /// use ruru::{Array, Object, VM};
    /// # VM::init();
    ///
    /// let array = Array::new();
    /// let another_array = Array::new();
    ///
    /// assert!(array.singleton_class() != another_array.singleton_class());
    /// ```
    ///
    /// Ruby:
    ///
    /// ```ruby
    /// array = []
    /// another_array = []
    ///
    /// array.singleton_class != another_array.singleton_class
    /// ```
    ///
    /// ### Modifying singleton class
    ///
    /// ```
    /// use ruru::{Array, Object, VM};
    /// # VM::init();
    ///
    /// let array = Array::new();
    /// let another_array = Array::new();
    ///
    /// array.singleton_class().define(|itself| {
    ///     itself.attr_reader("modified");
    /// });
    ///
    /// assert!(array.respond_to("modified"));
    /// assert!(!another_array.respond_to("modified"));
    /// ```
    ///
    /// Ruby:
    ///
    /// ```ruby
    /// array = []
    ///
    /// class << array
    ///   attr_reader :modified
    /// end
    ///
    /// array.respond_to?(:modified)
    /// ```
    fn singleton_class(&self) -> Class {
        let class = class::singleton_class(self.value());

        Class::from(class)
    }

    /// Wraps calls to the object.
    ///
    /// Mostly used to have Ruby-like class definition DSL.
    ///
    /// # Examples
    ///
    /// ### Defining class
    ///
    /// ```no_run
    /// #[macro_use] extern crate ruru;
    ///
    /// use ruru::{Class, Fixnum, Object, RString};
    ///
    /// class!(Hello);
    /// class!(Nested);
    ///
    /// methods!(
    ///     Hello,
    ///     itself,
    ///
    ///     fn greeting() -> RString {
    ///         RString::new("Greeting from class")
    ///     }
    ///
    ///     fn many_greetings() -> RString {
    ///         RString::new("Many greetings from instance")
    ///     }
    /// );
    ///
    /// methods!(
    ///     Nested,
    ///     itself,
    ///
    ///     fn nested_greeting() -> RString {
    ///         RString::new("Greeting from nested class")
    ///     }
    /// );
    ///
    /// fn main() {
    ///     Class::new("Hello", None).define(|itself| {
    ///         itself.attr_reader("reader");
    ///
    ///         itself.def_self("greeting", greeting);
    ///         itself.def("many_greetings", many_greetings);
    ///
    ///         itself.define_nested_class("Nested", None).define(|itself| {
    ///             itself.def_self("nested_greeting", nested_greeting);
    ///         });
    ///     });
    /// }
    /// ```
    ///
    /// Ruby:
    ///
    /// ```ruby
    /// class Hello
    ///   attr_reader :reader
    ///
    ///   def self.greeting
    ///     'Greeting from class'
    ///   end
    ///
    ///   def many_greetings
    ///     'Many greetings from instance'
    ///   end
    ///
    ///   class Nested
    ///     def self.nested_greeting
    ///       'Greeting from nested class'
    ///     end
    ///   end
    /// end
    /// ```
    ///
    /// ### Defining singleton method for an object
    ///
    /// ```
    /// #[macro_use] extern crate ruru;
    ///
    /// use ruru::{AnyObject, Class, Fixnum, Object, RString, VM};
    ///
    /// methods!(
    ///     RString,
    ///     itself,
    ///
    ///     fn greeting() -> RString {
    ///         RString::new("Greeting!")
    ///     }
    /// );
    ///
    /// fn main() {
    ///     # VM::init();
    ///     let mut string = RString::new("Some string");
    ///
    ///     // The same can be done by modifying `string.singleton_class()`
    ///     // or using `string.define_singleton_method("greeting", greeting)`
    ///     string.define(|itself| {
    ///         itself.define_singleton_method("greeting", greeting);
    ///     });
    ///
    ///     assert!(string.respond_to("greeting"));
    /// }
    /// ```
    ///
    /// Ruby:
    ///
    /// ```ruby
    /// string = "Some string"
    ///
    /// class << string
    ///   def greeting
    ///     'Greeting!'
    ///   end
    /// end
    ///
    /// string.respond_to?("greeting")
    /// ```
    fn define<F: Fn(&mut Self)>(&mut self, f: F) -> &Self {
        f(self);

        self
    }

    /// Defines an instance method for the given class or object.
    ///
    /// Use `methods!` macro to define a `callback`.
    ///
    /// You can also use `def()` alias for this function combined with `Class::define()` a for
    /// nicer DSL.
    ///
    /// # Panics
    ///
    /// Ruby can raise an exception if you try to define instance method directly on an instance
    /// of some class (like `Fixnum`, `String`, `Array` etc).
    ///
    /// Use this method only on classes (or singleton classes of objects).
    ///
    /// # Examples
    ///
    /// ### The famous String#blank? method
    ///
    /// ```rust
    /// #[macro_use] extern crate ruru;
    ///
    /// use ruru::{Boolean, Class, Object, RString, VM};
    ///
    /// methods!(
    ///    RString,
    ///    itself,
    ///
    ///    fn is_blank() -> Boolean {
    ///        Boolean::new(itself.to_string().chars().all(|c| c.is_whitespace()))
    ///    }
    /// );
    ///
    /// fn main() {
    ///     # VM::init();
    ///     Class::from_existing("String").define(|itself| {
    ///         itself.def("blank?", is_blank);
    ///     });
    /// }
    /// ```
    ///
    /// Ruby:
    ///
    /// ```ruby
    /// class String
    ///   def blank?
    ///     # simplified
    ///     self.chars.all? { |c| c == ' ' }
    ///   end
    /// end
    /// ```
    ///
    /// ### Receiving arguments
    ///
    /// Raise `Fixnum` to the power of `exp`.
    ///
    /// ```rust
    /// #[macro_use] extern crate ruru;
    ///
    /// use std::error::Error;
    ///
    /// use ruru::{Class, Fixnum, Object, VM};
    ///
    /// methods!(
    ///     Fixnum,
    ///     itself,
    ///
    ///     fn pow(exp: Fixnum) -> Fixnum {
    ///         // `exp` is not a valid `Fixnum`, raise an exception
    ///         if let Err(ref error) = exp {
    ///             VM::raise(error.to_exception(), error.description());
    ///         }
    ///
    ///         // We can safely unwrap here, because an exception was raised if `exp` is `Err`
    ///         let exp = exp.unwrap().to_i64() as u32;
    ///
    ///         Fixnum::new(itself.to_i64().pow(exp))
    ///     }
    ///
    ///     fn pow_with_default_argument(exp: Fixnum) -> Fixnum {
    ///         let default_exp = 0;
    ///         let exp = exp.map(|exp| exp.to_i64()).unwrap_or(default_exp);
    ///
    ///         let result = itself.to_i64().pow(exp as u32);
    ///
    ///         Fixnum::new(result)
    ///     }
    /// );
    ///
    /// fn main() {
    ///     # VM::init();
    ///     Class::from_existing("Fixnum").define(|itself| {
    ///         itself.def("pow", pow);
    ///         itself.def("pow_with_default_argument", pow_with_default_argument);
    ///     });
    /// }
    /// ```
    ///
    /// Ruby:
    ///
    /// ```ruby
    /// class Fixnum
    ///   def pow(exp)
    ///     raise ArgumentError unless exp.is_a?(Fixnum)
    ///
    ///     self ** exp
    ///   end
    ///
    ///   def pow_with_default_argument(exp)
    ///     default_exp = 0
    ///     exp = default_exp unless exp.is_a?(Fixnum)
    ///
    ///     self ** exp
    ///   end
    /// end
    /// ```
    fn define_method<I: Object, O: Object>(&mut self, name: &str, callback: Callback<I, O>) {
        class::define_method(self.value(), name, callback);
    }

    /// Defines a class method for given class or singleton method for object.
    ///
    /// Use `methods!` macro to define a `callback`.
    ///
    /// You can also use `def_self()` alias for this function combined with `Class::define()` a for
    /// nicer DSL.
    ///
    /// # Examples
    ///
    /// ### Defining a class method
    ///
    /// ```
    /// #[macro_use] extern crate ruru;
    ///
    /// use std::error::Error;
    ///
    /// use ruru::{Class, Object, RString, Symbol, VM};
    ///
    /// methods!(
    ///     Symbol,
    ///     itself,
    ///
    ///     fn from_string(string: RString) -> Symbol {
    ///         // `string` is not a valid `String`, raise an exception
    ///         if let Err(ref error) = string {
    ///             VM::raise(error.to_exception(), error.description());
    ///         }
    ///
    ///         Symbol::new(&string.unwrap().to_string())
    ///     }
    /// );
    ///
    /// fn main() {
    ///     # VM::init();
    ///     Class::from_existing("Symbol").define(|itself| {
    ///         itself.def_self("from_string", from_string);
    ///     });
    /// }
    /// ```
    ///
    /// Ruby:
    ///
    /// ```ruby
    /// class Symbol
    ///   def self.from_string(string)
    ///     raise ArgumentError unless string.is_a?(String)
    ///
    ///     # simplified
    ///     string.to_sym
    ///   end
    /// end
    /// ```
    ///
    /// ### Defining a singleton method for an object
    ///
    /// ```
    /// #[macro_use] extern crate ruru;
    ///
    /// use ruru::{AnyObject, Class, Fixnum, Object, RString, VM};
    ///
    /// methods!(
    ///     RString,
    ///     itself,
    ///
    ///     fn greeting() -> RString {
    ///         RString::new("Greeting!")
    ///     }
    /// );
    ///
    /// fn main() {
    ///     # VM::init();
    ///     let mut string = RString::new("Some string");
    ///
    ///     // The same can be done by modifying `string.singleton_class()`
    ///     // or using `string.define_singleton_method("greeting", greeting)`
    ///     string.define(|itself| {
    ///         itself.define_singleton_method("greeting", greeting);
    ///     });
    ///
    ///     assert!(string.respond_to("greeting"));
    /// }
    /// ```
    ///
    /// Ruby:
    ///
    /// ```ruby
    ///
    /// string = "Some string"
    ///
    /// class << string
    ///   def greeting
    ///     'Greeting!'
    ///   end
    /// end
    ///
    /// string.respond_to?("greeting")
    /// ```
    fn define_singleton_method<I: Object, O: Object>(&mut self,
                                                     name: &str,
                                                     callback: Callback<I, O>) {
        class::define_singleton_method(self.value(), name, callback);
    }

    /// An alias for `define_method` (similar to Ruby syntax `def some_method`).
    fn def<I: Object, O: Object>(&mut self, name: &str, callback: Callback<I, O>) {
        self.define_method(name, callback);
    }

    /// An alias for `define_singleton_method` (similar to Ruby `def self.some_method`).
    fn def_self<I: Object, O: Object>(&mut self, name: &str, callback: Callback<I, O>) {
        self.define_singleton_method(name, callback);
    }

    /// Calls a given method on an object similarly to Ruby `Object#send` method
    ///
    /// # Examples
    ///
    /// ```
    /// use ruru::{Array, Fixnum, Object, RString, VM};
    /// # VM::init();
    ///
    /// let array = Array::new().push(Fixnum::new(1));
    /// let array_to_str =
    ///     array
    ///         .send("to_s", vec![])
    ///         .try_convert_to::<RString>()
    ///         .unwrap()
    ///         .to_string();
    ///
    /// assert_eq!(array_to_str, "[1]".to_string());
    /// ```
    fn send(&self, method: &str, arguments: Vec<AnyObject>) -> AnyObject {
        let (argc, argv) = util::create_arguments(arguments);

        let result = binding_util::call_method(self.value(), method, argc, argv.as_ptr());

        AnyObject::from(result)
    }

    /// Checks whether the object responds to given method
    ///
    /// # Examples
    ///
    /// ```
    /// use ruru::{Array, Object, VM};
    /// # VM::init();
    ///
    /// let array = Array::new();
    ///
    /// assert!(array.respond_to("push"));
    /// assert!(!array.respond_to("something_else"));
    /// ```
    fn respond_to(&self, method: &str) -> bool {
        class::respond_to(self.value(), method)
    }

    /// Checks whether the object is `nil`
    ///
    /// # Examples
    ///
    /// ```
    /// use ruru::{Hash, NilClass, Object, VM};
    /// # VM::init();
    ///
    /// assert!(NilClass::new().is_nil());
    /// assert!(!Hash::new().is_nil());
    /// ```
    ///
    /// Ruby:
    ///
    /// ```ruby
    /// nil.nil? == true
    /// {}.nil? == false
    /// ```
    fn is_nil(&self) -> bool {
        self.value().is_nil()
    }

    /// Converts struct to `AnyObject`
    ///
    /// See docs for `AnyObject` class for more details.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruru::{Array, Fixnum, Object, VM};
    /// # VM::init();
    ///
    /// let array = Array::new().push(Fixnum::new(1));
    /// let args = vec![Fixnum::new(1).to_any_object()];
    /// let index =
    ///     array
    ///         .send("find_index", args)
    ///         .try_convert_to::<Fixnum>();
    ///
    /// assert_eq!(index, Ok(Fixnum::new(0)));
    /// ```
    fn to_any_object(&self) -> AnyObject {
        AnyObject::from(self.value())
    }

    /// Gets an instance variable of object
    ///
    /// # Examples
    ///
    /// ```
    /// #[macro_use]
    /// extern crate ruru;
    ///
    /// use ruru::{AnyObject, Class, Fixnum, Object, VM};
    /// use ruru::types::Argc;
    ///
    /// class!(Counter);
    ///
    /// methods!(
    ///     Counter,
    ///     itself,
    ///
    ///     fn counter_initialize() -> AnyObject {
    ///         itself.instance_variable_set("@state", Fixnum::new(0))
    ///     }
    ///
    ///     fn counter_increment() -> AnyObject {
    ///         // Using unsafe conversion, because we are sure that `@state` is always a `Fixnum`
    ///         // and we don't provide an interface to set the value externally
    ///         let state = unsafe {
    ///             itself.instance_variable_get("@state").to::<Fixnum>().to_i64()
    ///         };
    ///
    ///         itself.instance_variable_set("@state", Fixnum::new(state + 1))
    ///     }
    ///
    ///     fn counter_state() -> Fixnum {
    ///         unsafe { itself.instance_variable_get("@state").to::<Fixnum>() }
    ///     }
    /// );
    ///
    /// fn main() {
    ///     # VM::init();
    ///     let counter = Class::new("Counter", None).define(|itself| {
    ///         itself.def("initialize", counter_initialize);
    ///         itself.def("increment!", counter_increment);
    ///         itself.def("state", counter_state);
    ///     }).new_instance(vec![]);
    ///
    ///     counter.send("increment!", vec![]);
    ///
    ///     let new_state = counter.send("state", vec![]).try_convert_to::<Fixnum>();
    ///
    ///     assert_eq!(new_state, Ok(Fixnum::new(1)));
    /// }
    /// ```
    fn instance_variable_get(&self, variable: &str) -> AnyObject {
        let result = class::instance_variable_get(self.value(), variable);

        AnyObject::from(result)
    }

    /// Sets an instance variable for object
    ///
    /// # Examples
    ///
    /// ```
    /// #[macro_use]
    /// extern crate ruru;
    ///
    /// use ruru::{AnyObject, Class, Fixnum, Object, VM};
    /// use ruru::types::Argc;
    ///
    /// class!(Counter);
    ///
    /// methods!(
    ///     Counter,
    ///     itself,
    ///
    ///     fn counter_initialize() -> AnyObject {
    ///         itself.instance_variable_set("@state", Fixnum::new(0))
    ///     }
    ///
    ///     fn counter_increment() -> AnyObject {
    ///         // Using unsafe conversion, because we are sure that `@state` is always a `Fixnum`
    ///         // and we don't provide an interface to set the value externally
    ///         let state = unsafe {
    ///             itself.instance_variable_get("@state").to::<Fixnum>().to_i64()
    ///         };
    ///
    ///         itself.instance_variable_set("@state", Fixnum::new(state + 1))
    ///     }
    ///
    ///     fn counter_state() -> Fixnum {
    ///         unsafe { itself.instance_variable_get("@state").to::<Fixnum>() }
    ///     }
    /// );
    ///
    /// fn main() {
    ///     # VM::init();
    ///     let counter = Class::new("Counter", None).define(|itself| {
    ///         itself.def("initialize", counter_initialize);
    ///         itself.def("increment!", counter_increment);
    ///         itself.def("state", counter_state);
    ///     }).new_instance(vec![]);
    ///
    ///     counter.send("increment!", vec![]);
    ///
    ///     let new_state = counter.send("state", vec![]).try_convert_to::<Fixnum>();
    ///
    ///     assert_eq!(new_state, Ok(Fixnum::new(1)));
    /// }
    /// ```
    fn instance_variable_set<T: Object>(&mut self, variable: &str, value: T) -> AnyObject {
        let result = class::instance_variable_set(self.value(), variable, value.value());

        AnyObject::from(result)
    }

    /// Unsafely casts current object to the specified Ruby type
    ///
    /// This operation in unsafe, because it does not perform any validations on the object, but
    /// it is faster than `try_convert_to()`.
    ///
    /// Use it when:
    ///
    ///  - you own the Ruby code which passes the object to Rust;
    ///  - you are sure that the object always has correct type;
    ///  - Ruby code has a good test coverage.
    ///
    /// This function is used by `unsafe_methods!` macro for argument casting.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruru::{AnyObject, Fixnum, Object, VM};
    /// # VM::init();
    ///
    /// let fixnum_as_any_object = Fixnum::new(1).to_any_object();
    ///
    /// let fixnum = unsafe { fixnum_as_any_object.to::<Fixnum>() };
    ///
    /// assert_eq!(fixnum.to_i64(), 1);
    /// ```
    unsafe fn to<T: Object>(&self) -> T {
        T::from(self.value())
    }

    /// Safely casts current object to the specified Ruby type
    ///
    /// This function is used by `methods!` macro for argument casting.
    ///
    /// See documentation for `VerifiedObject` trait to enable safe conversions
    /// for custom classes.
    ///
    /// # Examples
    ///
    /// ### Basic conversions
    ///
    /// ```
    /// use ruru::result::Error;
    /// use ruru::{Fixnum, Object, RString, VM};
    /// # VM::init();
    ///
    /// let fixnum_as_any_object = Fixnum::new(1).to_any_object();
    /// let converted_fixnum = fixnum_as_any_object.try_convert_to::<Fixnum>();
    ///
    /// assert_eq!(converted_fixnum, Ok(Fixnum::new(1)));
    ///
    /// let string = RString::new("string");
    /// let string_as_fixnum = string.try_convert_to::<Fixnum>();
    /// let expected_error = Error::TypeError("Error converting to Fixnum".to_string());
    ///
    /// assert_eq!(string_as_fixnum, Err(expected_error));
    /// ```
    ///
    /// ### Method arguments
    ///
    /// To launch a server in Rust, you plan to write a simple `Server` class
    ///
    /// ```ruby
    /// class Server
    ///   def start(address)
    ///     # ...
    ///   end
    /// end
    /// ```
    ///
    /// The `address` must be `Hash` with the following structure:
    ///
    /// ```ruby
    /// {
    ///   host: 'localhost',
    ///   port: 8080,
    /// }
    /// ```
    ///
    /// You want to extract port from it. Default port is `8080` in case when:
    ///
    ///  - `address` is not a `Hash`
    ///  - `address[:port]` is not present
    ///  - `address[:port]` is not a `Fixnum`
    ///
    /// ```no_run
    /// #[macro_use]
    /// extern crate ruru;
    ///
    /// use ruru::{Class, Fixnum, Hash, NilClass, Object, Symbol, VM};
    ///
    /// class!(Server);
    ///
    /// methods!(
    ///     Server,
    ///     itself,
    ///
    ///     fn start(address: Hash) -> NilClass {
    ///         let default_port = 8080;
    ///
    ///         let port = address
    ///             .map(|hash| hash.at(Symbol::new("port")))
    ///             .and_then(|port| port.try_convert_to::<Fixnum>())
    ///             .map(|port| port.to_i64())
    ///             .unwrap_or(default_port);
    ///
    ///         // Start server...
    ///
    ///         NilClass::new()
    ///     }
    /// );
    ///
    /// fn main() {
    ///     # VM::init();
    ///     Class::new("Server", None).define(|itself| {
    ///         itself.def("start", start);
    ///     });
    /// }
    /// ```
    ///
    /// Ruby:
    ///
    /// ```ruby
    /// class Server
    ///   def start(address)
    ///     default_port = 8080
    ///
    ///     port =
    ///       if address.is_a?(Hash) && address[:port].is_a?(Fixnum)
    ///         address[:port]
    ///       else
    ///         default_port
    ///       end
    ///
    ///     # Start server...
    ///   end
    /// end
    /// ```
    fn try_convert_to<T: VerifiedObject>(&self) -> Result<T> {
        if T::is_correct_type(self) {
            let converted_object = unsafe { self.to::<T>() };

            Ok(converted_object)
        } else {
            Err(Error::TypeError(T::error_message().to_string()))
        }
    }

    /// Determines the value type of the object
    ///
    /// # Example
    ///
    /// ```
    /// use ruru::{AnyObject, Fixnum, Object, VM};
    /// use ruru::types::ValueType;
    /// # VM::init();
    ///
    /// let any_object = Fixnum::new(1).to_any_object();
    ///
    /// assert_eq!(any_object.ty(), ValueType::Fixnum);
    /// ```
    fn ty(&self) -> ValueType {
        self.value().ty()
    }
}
