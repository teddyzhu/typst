use std::fmt::{self, Debug, Formatter, Write};
use std::str::FromStr;

use ecow::{eco_vec, EcoVec};
use smallvec::{smallvec, SmallVec};
use typst::eval::Tracer;
use typst::model::DelayedErrors;

use super::{FigureElem, HeadingElem, Numbering, NumberingPattern};
use crate::layout::PageElem;
use crate::math::EquationElem;
use crate::prelude::*;

/// Counts through pages, elements, and more.
///
/// With the counter function, you can access and modify counters for pages,
/// headings, figures, and more. Moreover, you can define custom counters for
/// other things you want to count.
///
/// ## Displaying a counter { #displaying }
/// To display the current value of the heading counter, you call the `counter`
/// function with the `key` set to `heading` and then call the `display` method
/// on the counter. To see any output, you also have to enable heading
/// [numbering]($func/heading.numbering).
///
/// The `display` method optionally takes an argument telling it how to format
/// the counter. This can be a [numbering pattern or a
/// function]($func/numbering).
///
/// ```example
/// #set heading(numbering: "1.")
///
/// = Introduction
/// Some text here.
///
/// = Background
/// The current value is:
/// #counter(heading).display()
///
/// Or in roman numerals:
/// #counter(heading).display("I")
/// ```
///
/// ## Modifying a counter { #modifying }
/// To modify a counter, you can use the `step` and `update` methods:
///
/// - The `step` method increases the value of the counter by one. Because
///   counters can have multiple levels (in the case of headings for sections,
///   subsections, and so on), the `step` method optionally takes a `level`
///   argument. If given, the counter steps at the given depth.
///
/// - The `update` method allows you to arbitrarily modify the counter. In its
///   basic form, you give it an integer (or multiple for multiple levels). For
///   more flexibility, you can instead also give it a function that gets the
///   current value and returns a new value.
///
/// The heading counter is stepped before the heading is displayed, so
/// `Analysis` gets the number seven even though the counter is at six after the
/// second update.
///
/// ```example
/// #set heading(numbering: "1.")
///
/// = Introduction
/// #counter(heading).step()
///
/// = Background
/// #counter(heading).update(3)
/// #counter(heading).update(n => n * 2)
///
/// = Analysis
/// Let's skip 7.1.
/// #counter(heading).step(level: 2)
///
/// == Analysis
/// Still at #counter(heading).display().
/// ```
///
/// ## Custom counters { #custom-counters }
/// To define your own counter, call the `counter` function with a string as a
/// key. This key identifies the counter globally.
///
/// ```example
/// #let mine = counter("mycounter")
/// #mine.display() \
/// #mine.step()
/// #mine.display() \
/// #mine.update(c => c * 3)
/// #mine.display() \
/// ```
///
/// ## How to step { #how-to-step }
/// When you define and use a custom counter, in general, you should first step
/// the counter and then display it. This way, the stepping behaviour of a
/// counter can depend on the element it is stepped for. If you were writing a
/// counter for, let's say, theorems, your theorem's definition would thus first
/// include the counter step and only then display the counter and the theorem's
/// contents.
///
/// ```example
/// #let c = counter("theorem")
/// #let theorem(it) = block[
///   #c.step()
///   *Theorem #c.display():* #it
/// ]
///
/// #theorem[$1 = 1$]
/// #theorem[$2 < 3$]
/// ```
///
/// The rationale behind this is best explained on the example of the heading
/// counter: An update to the heading counter depends on the heading's level.
/// By stepping directly before the heading, we can correctly step from `1` to
/// `1.1` when encountering a level 2 heading. If we were to step after the
/// heading, we wouldn't know what to step to.
///
/// Because counters should always be stepped before the elements they count,
/// they always start at zero. This way, they are at one for the first display
/// (which happens after the first step).
///
/// ## Page counter { #page-counter }
/// The page counter is special. It is automatically stepped at each pagebreak.
/// But like other counters, you can also step it manually. For example, you
/// could have Roman page numbers for your preface, then switch to Arabic page
/// numbers for your main content and reset the page counter to one.
///
/// ```example
/// >>> #set page(
/// >>>   height: 100pt,
/// >>>   margin: (bottom: 24pt, rest: 16pt),
/// >>> )
/// #set page(numbering: "(i)")
///
/// = Preface
/// The preface is numbered with
/// roman numerals.
///
/// #set page(numbering: "1 / 1")
/// #counter(page).update(1)
///
/// = Main text
/// Here, the counter is reset to one.
/// We also display both the current
/// page and total number of pages in
/// Arabic numbers.
/// ```
///
/// ## Time travel { #time-travel }
/// Counters can travel through time! You can find out the final value of the
/// counter before it is reached and even determine what the value was at any
/// particular location in the document.
///
/// ```example
/// #let mine = counter("mycounter")
///
/// = Values
/// #locate(loc => {
///   let start-val = mine.at(loc)
///   let elements = query(<intro>, loc)
///   let intro-val = mine.at(
///     elements.first().location()
///   )
///   let final-val = mine.final(loc)
///   [Starts as: #start-val \
///    Value at intro is: #intro-val \
///    Final value is: #final-val \ ]
/// })
///
/// #mine.update(n => n + 3)
///
/// = Introduction <intro>
/// #lorem(10)
///
/// #mine.step()
/// #mine.step()
/// ```
///
/// Let's dissect what happens in the example above:
///
/// - We call [`locate`]($func/locate) to get access to the current location in
///   the document. We then pass this location to our counter's `at` method to
///   get its value at the current location. The `at` method always returns an
///   array because counters can have multiple levels. As the counter starts at
///   one, the first value is thus `{(1,)}`.
///
/// - We now [`query`]($func/query) the document for all elements with the
///   `{<intro>}` label. The result is an array from which we extract the first
///   (and only) element's [location]($type/content.location). We then look up
///   the value of the counter at that location. The first update to the counter
///   sets it to `{1 + 3 = 4}`. At the introduction heading, the value is thus
///   `{(4,)}`.
///
/// - Last but not least, we call the `final` method on the counter. It tells us
///   what the counter's value will be at the end of the document. We also need
///   to give it a location to prove that we are inside of a `locate` call, but
///   which one doesn't matter. After the heading follow two calls to `step()`,
///   so the final value is `{(6,)}`.
///
/// ## Other kinds of state { #other-state }
/// The `counter` function is closely related to [state]($func/state) function.
/// Read its documentation for more details on state management in Typst and
/// why it doesn't just use normal variables for counters.
///
/// ## Methods
/// ### display()
/// Displays the value of the counter.
///
/// - numbering: string or function (positional)
///   A [numbering pattern or a function]($func/numbering), which specifies how
///   to display the counter. If given a function, that function receives each
///   number of the counter as a separate argument. If the amount of numbers
///   varies, e.g. for the heading argument, you can use an
///   [argument sink]($type/arguments).
///
///   If this is omitted, displays the counter with the numbering style for the
///   counted element or with the pattern `{"1.1"}` if no such style exists.
///
/// - both: boolean (named)
///   If enabled, displays the current and final top-level count together. Both
///   can be styled through a single numbering pattern. This is used by the page
///   numbering property to display the current and total number of pages when a
///   pattern like `{"1 / 1"}` is given.
///
/// - returns: content
///
/// ### step()
/// Increases the value of the counter by one.
///
/// The update will be in effect at the position where the returned content is
/// inserted into the document. If you don't put the output into the document,
/// nothing happens! This would be the case, for example, if you write
/// `{let _ = counter(page).step()}`. Counter updates are always applied in
/// layout order and in that case, Typst wouldn't know when to step the counter.
///
/// - level: integer (named)
///   The depth at which to step the counter. Defaults to `{1}`.
///
/// - returns: content
///
/// ### update()
/// Updates the value of the counter.
///
/// Just like with `step`, the update only occurs if you put the resulting
/// content into the document.
///
/// - value: integer or array or function (positional, required)
///   If given an integer or array of integers, sets the counter to that value.
///   If given a function, that function receives the previous counter value
///   (with each number as a separate argument) and has to return the new
///   value (integer or array).
///
/// - returns: content
///
/// ### at()
/// Gets the value of the counter at the given location. Always returns an
/// array of integers, even if the counter has just one number.
///
/// - location: location (positional, required)
///   The location at which the counter value should be retrieved. A suitable
///   location can be retrieved from [`locate`]($func/locate) or
///   [`query`]($func/query).
///
/// - returns: array
///
/// ### final()
/// Gets the value of the counter at the end of the document. Always returns an
/// array of integers, even if the counter has just one number.
///
/// - location: location (positional, required)
///   Can be any location. Why is it required then? Typst has to evaluate parts
///   of your code multiple times to determine all counter values. By only
///   allowing this method within [`locate`]($func/locate) calls, the amount of
///   code that can depend on the method's result is reduced. If you could call
///   `final` directly at the top level of a module, the evaluation of the whole
///   module and its exports could depend on the counter's value.
///
/// - returns: array
///
/// Display: Counter
/// Category: meta
#[func]
pub fn counter(
    /// The key that identifies this counter.
    ///
    /// - If it is a string, creates a custom counter that is only affected by
    ///   manual updates,
    /// - If this is a `{<label>}`, counts through all elements with that label,
    /// - If this is an element function or selector, counts through its elements,
    /// - If this is the [`page`]($func/page) function, counts through pages.
    key: CounterKey,
) -> Counter {
    Counter::new(key)
}

/// Counts through pages, elements, and more.
#[derive(Clone, PartialEq, Hash)]
pub struct Counter(CounterKey);

impl Counter {
    /// Create a new counter from a key.
    pub fn new(key: CounterKey) -> Self {
        Self(key)
    }

    /// The counter for the given element.
    pub fn of(func: ElemFunc) -> Self {
        Self::new(CounterKey::Selector(Selector::Elem(func, None)))
    }

    /// Call a method on counter.
    #[tracing::instrument(skip(vm))]
    pub fn call_method(
        self,
        vm: &mut Vm,
        method: &str,
        mut args: Args,
        span: Span,
    ) -> SourceResult<Value> {
        let value = match method {
            "display" => self
                .display(args.eat()?, args.named("both")?.unwrap_or(false))
                .into_value(),
            "step" => self
                .update(CounterUpdate::Step(
                    args.named("level")?.unwrap_or(NonZeroUsize::ONE),
                ))
                .into_value(),
            "update" => self.update(args.expect("value or function")?).into_value(),
            "at" => self.at(&mut vm.vt, args.expect("location")?)?.into_value(),
            "final" => self.final_(&mut vm.vt, args.expect("location")?)?.into_value(),
            _ => bail!(span, "type counter has no method `{}`", method),
        };
        args.finish()?;
        Ok(value)
    }

    /// Display the current value of the counter.
    pub fn display(self, numbering: Option<Numbering>, both: bool) -> Content {
        DisplayElem::new(self, numbering, both).pack()
    }

    /// Get the value of the state at the given location.
    pub fn at(&self, vt: &mut Vt, location: Location) -> SourceResult<CounterState> {
        let sequence = self.sequence(vt)?;
        let offset = vt.introspector.query(&self.selector().before(location, true)).len();
        let (mut state, page) = sequence[offset].clone();
        if self.is_page() {
            let delta = vt.introspector.page(location).get().saturating_sub(page.get());
            state.step(NonZeroUsize::ONE, delta);
        }

        Ok(state)
    }

    /// Get the value of the state at the final location.
    pub fn final_(&self, vt: &mut Vt, _: Location) -> SourceResult<CounterState> {
        let sequence = self.sequence(vt)?;
        let (mut state, page) = sequence.last().unwrap().clone();
        if self.is_page() {
            let delta = vt.introspector.pages().get().saturating_sub(page.get());
            state.step(NonZeroUsize::ONE, delta);
        }
        Ok(state)
    }

    /// Get the current and final value of the state combined in one state.
    pub fn both(&self, vt: &mut Vt, location: Location) -> SourceResult<CounterState> {
        let sequence = self.sequence(vt)?;
        let offset = vt
            .introspector
            .query(&Selector::before(self.selector(), location, true))
            .len();
        let (mut at_state, at_page) = sequence[offset].clone();
        let (mut final_state, final_page) = sequence.last().unwrap().clone();
        if self.is_page() {
            let at_delta =
                vt.introspector.page(location).get().saturating_sub(at_page.get());
            at_state.step(NonZeroUsize::ONE, at_delta);
            let final_delta =
                vt.introspector.pages().get().saturating_sub(final_page.get());
            final_state.step(NonZeroUsize::ONE, final_delta);
        }
        Ok(CounterState(smallvec![at_state.first(), final_state.first()]))
    }

    /// Produce content that performs a state update.
    pub fn update(self, update: CounterUpdate) -> Content {
        UpdateElem::new(self.0, update).pack()
    }

    /// Produce the whole sequence of counter states.
    ///
    /// This has to happen just once for all counters, cutting down the number
    /// of counter updates from quadratic to linear.
    fn sequence(
        &self,
        vt: &mut Vt,
    ) -> SourceResult<EcoVec<(CounterState, NonZeroUsize)>> {
        self.sequence_impl(
            vt.world,
            vt.introspector,
            vt.locator.track(),
            TrackedMut::reborrow_mut(&mut vt.delayed),
            TrackedMut::reborrow_mut(&mut vt.tracer),
        )
    }

    /// Memoized implementation of `sequence`.
    #[comemo::memoize]
    fn sequence_impl(
        &self,
        world: Tracked<dyn World + '_>,
        introspector: Tracked<Introspector>,
        locator: Tracked<Locator>,
        delayed: TrackedMut<DelayedErrors>,
        tracer: TrackedMut<Tracer>,
    ) -> SourceResult<EcoVec<(CounterState, NonZeroUsize)>> {
        let mut locator = Locator::chained(locator);
        let mut vt = Vt {
            world,
            introspector,
            locator: &mut locator,
            delayed,
            tracer,
        };
        let mut state = CounterState(match &self.0 {
            // special case, because pages always start at one.
            CounterKey::Page => smallvec![1],
            _ => smallvec![0],
        });
        let mut page = NonZeroUsize::ONE;
        let mut stops = eco_vec![(state.clone(), page)];

        for elem in introspector.query(&self.selector()) {
            if self.is_page() {
                let prev = page;
                page = introspector.page(elem.location().unwrap());

                let delta = page.get() - prev.get();
                if delta > 0 {
                    state.step(NonZeroUsize::ONE, delta);
                }
            }

            if let Some(update) = match elem.to::<UpdateElem>() {
                Some(elem) => Some(elem.update()),
                None => match elem.with::<dyn Count>() {
                    Some(countable) => countable.update(),
                    None => Some(CounterUpdate::Step(NonZeroUsize::ONE)),
                },
            } {
                state.update(&mut vt, update)?;
            }

            stops.push((state.clone(), page));
        }

        Ok(stops)
    }

    /// The selector relevant for this counter's updates.
    fn selector(&self) -> Selector {
        let mut selector =
            Selector::Elem(UpdateElem::func(), Some(dict! { "key" => self.0.clone() }));

        if let CounterKey::Selector(key) = &self.0 {
            selector = Selector::Or(eco_vec![selector, key.clone()]);
        }

        selector
    }

    /// Whether this is the page counter.
    fn is_page(&self) -> bool {
        self.0 == CounterKey::Page
    }
}

impl Debug for Counter {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("counter(")?;
        self.0.fmt(f)?;
        f.write_char(')')
    }
}

cast! {
    type Counter: "counter",
}

/// Identifies a counter.
#[derive(Clone, PartialEq, Hash)]
pub enum CounterKey {
    /// The page counter.
    Page,
    /// Counts elements matching the given selectors. Only works for locatable
    /// elements or labels.
    Selector(Selector),
    /// Counts through manual counters with the same key.
    Str(Str),
}

cast! {
    CounterKey,
    self => match self {
        Self::Page => PageElem::func().into_value(),
        Self::Selector(v) => v.into_value(),
        Self::Str(v) => v.into_value(),
    },
    v: Str => Self::Str(v),
    v: Label => Self::Selector(Selector::Label(v)),
    v: ElemFunc => {
        if v == PageElem::func() {
            Self::Page
        } else {
            Self::Selector(LocatableSelector::from_value(v.into_value())?.0)
        }
    },
    v: LocatableSelector => Self::Selector(v.0),
}

impl Debug for CounterKey {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Page => f.pad("page"),
            Self::Selector(selector) => selector.fmt(f),
            Self::Str(str) => str.fmt(f),
        }
    }
}

/// An update to perform on a counter.
#[derive(Clone, PartialEq, Hash)]
pub enum CounterUpdate {
    /// Set the counter to the specified state.
    Set(CounterState),
    /// Increase the number for the given level by one.
    Step(NonZeroUsize),
    /// Apply the given function to the counter's state.
    Func(Func),
}

impl Debug for CounterUpdate {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("..")
    }
}

cast! {
    type CounterUpdate: "counter update",
    v: CounterState => Self::Set(v),
    v: Func => Self::Func(v),
}

/// Elements that have special counting behaviour.
pub trait Count {
    /// Get the counter update for this element.
    fn update(&self) -> Option<CounterUpdate>;
}

/// Counts through elements with different levels.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct CounterState(pub SmallVec<[usize; 3]>);

impl CounterState {
    /// Advance the counter and return the numbers for the given heading.
    pub fn update(&mut self, vt: &mut Vt, update: CounterUpdate) -> SourceResult<()> {
        match update {
            CounterUpdate::Set(state) => *self = state,
            CounterUpdate::Step(level) => self.step(level, 1),
            CounterUpdate::Func(func) => {
                *self = func.call_vt(vt, self.0.iter().copied())?.cast().at(func.span())?
            }
        }
        Ok(())
    }

    /// Advance the number of the given level by the specified amount.
    pub fn step(&mut self, level: NonZeroUsize, by: usize) {
        let level = level.get();

        if self.0.len() >= level {
            self.0[level - 1] = self.0[level - 1].saturating_add(by);
            self.0.truncate(level);
        }

        while self.0.len() < level {
            self.0.push(1);
        }
    }

    /// Get the first number of the state.
    pub fn first(&self) -> usize {
        self.0.first().copied().unwrap_or(1)
    }

    /// Display the counter state with a numbering.
    pub fn display(&self, vt: &mut Vt, numbering: &Numbering) -> SourceResult<Content> {
        Ok(numbering.apply_vt(vt, &self.0)?.display())
    }
}

cast! {
    CounterState,
    self => Value::Array(self.0.into_iter().map(IntoValue::into_value).collect()),
    num: usize => Self(smallvec![num]),
    array: Array => Self(array
        .into_iter()
        .map(Value::cast)
        .collect::<StrResult<_>>()?),
}

/// Executes a display of a state.
///
/// Display: State
/// Category: special
#[element(Locatable, Show)]
struct DisplayElem {
    /// The counter.
    #[required]
    counter: Counter,

    /// The numbering to display the counter with.
    #[required]
    numbering: Option<Numbering>,

    /// Whether to display both the current and final value.
    #[required]
    both: bool,
}

impl Show for DisplayElem {
    #[tracing::instrument(name = "DisplayElem::show", skip_all)]
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        Ok(vt.delayed(|vt| {
            let location = self.0.location().unwrap();
            let counter = self.counter();
            let numbering = self
                .numbering()
                .or_else(|| {
                    let CounterKey::Selector(Selector::Elem(func, _)) = counter.0 else {
                        return None;
                    };

                    if func == HeadingElem::func() {
                        HeadingElem::numbering_in(styles)
                    } else if func == FigureElem::func() {
                        FigureElem::numbering_in(styles)
                    } else if func == EquationElem::func() {
                        EquationElem::numbering_in(styles)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| NumberingPattern::from_str("1.1").unwrap().into());

            let state = if self.both() {
                counter.both(vt, location)?
            } else {
                counter.at(vt, location)?
            };

            state.display(vt, &numbering)
        }))
    }
}

/// Executes a display of a state.
///
/// Display: State
/// Category: special
#[element(Locatable, Show)]
struct UpdateElem {
    /// The key that identifies the counter.
    #[required]
    key: CounterKey,

    /// The update to perform on the counter.
    #[required]
    update: CounterUpdate,
}

impl Show for UpdateElem {
    #[tracing::instrument(name = "UpdateElem::show", skip(self))]
    fn show(&self, _: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}
