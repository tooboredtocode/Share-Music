/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

use crate::util::message_command::MessageCommand;

pub struct FindLinksCommand;

impl MessageCommand for FindLinksCommand {
    const NAME: &'static str = "Find Links";
}
