use statement::{State, Statement, Bindable, Readable};
use {Result, Value};

/// An iterator over rows.
pub struct Cursor<'l> {
    state: Option<State>,
    values: Option<Vec<Value>>,
    statement: Statement<'l>,
}

impl<'l> Cursor<'l> {
    /// Bind values to all parameters.
    pub fn bind(&mut self, values: &[Value]) -> Result<()> {
        try!(self.statement.reset());
        for (i, value) in values.iter().enumerate() {
            try!(self.statement.bind(i + 1, value));
        }
        Ok(())
    }

    /// Advance to the next row and read all columns.
    pub fn next(&mut self) -> Result<Option<&[Value]>> {
        match self.state {
            Some(State::Row) => {},
            Some(State::Done) => return Ok(None),
            _ => {
                self.state = Some(try!(self.statement.next()));
                return self.next();
            },
        }
        let values = match self.values.take() {
            Some(mut values) => {
                for (i, value) in values.iter_mut().enumerate() {
                    *value = try!(self.statement.read(i));
                }
                values
            },
            _ => {
                let count = self.statement.columns();
                let mut values = Vec::with_capacity(count);
                for i in 0..count {
                    values.push(try!(self.statement.read(i)));
                }
                values
            },
        };
        self.state = Some(try!(self.statement.next()));
        self.values = Some(values);
        Ok(Some(self.values.as_ref().unwrap()))
    }
}

#[inline]
pub fn new<'l>(statement: Statement<'l>) -> Result<Cursor<'l>> {
    Ok(Cursor { state: None, values: None, statement: statement })
}