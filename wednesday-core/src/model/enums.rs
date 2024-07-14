pub enum Feed<Event> {
    Next(Event),
    Unhealthy,
    Finished,
}
