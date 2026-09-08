# Configuration Format

Bubble's configuration file is written in TOML and composed of at least one _webhook_, optionally one or more _roles_ and _users_, and at least one _rule_.

## Webhooks
```
[webhooks]
main = "https://discord.com/api/webhooks/<id>/<token>"
rmb = "https://discord.com/api/webhooks/<id>/<token>"
```

A list of all webhook URLs to be used as output, each assigned to a key / name, which can be an arbitrary alphanumeric sequence.

## Roles
```
[roles]
rmb-team = "<role_id>"
endo-team = "<role_id>"
welcome-team = "<role_id>"
```

A list of all roles that can be pinged by the webhook, each assigned to a key / name. This section can be omitted if there are no roles to ping.

## Users
```
[users]
governor = "<user_id>"
delegate = "<user_id>"
```

A list of all users that can be pinged by the webhook, each assigned to a key / name. This section can be omitted if there are no users to ping.

## Rules

Each rule is composed of a set of _conditions_ that determine whether an event matches the rule, and _parameters_ for how to display that event, if it matches the rule.

Each rule has its own block, headed by `[rule.<key>]`. The key can be any sequence of alphanumeric characters, it is not relevant. You should, however, make sure that each rule has a unique key.

An example rule can be found below:
```
[rule.wa_join]
if_event = ["join"]
if_region = ["testregionia"]
if_wa = true
webhook = "main"
color = "#44BB12"
role_mentions = ["welcome-team", "endo-team"]
links = ["endorse"]
```

### Conditions

If there are no conditions specified, all supported events will match the rule.

**if_event (string[])**

If the event name is contained in this array, the event will match the rule (provided any other conditions match). Otherwise, it will not. If any element of this array starts with a "#" character, the rest of the string will indicate a regex pattern instead.

For example, `if_event = ["admit", "#wa-.+"]` will match any event whose name is either `admit` or matches the regex pattern `wa-.+` (for example "wa-floor").

Supported event names are:
- `admit`: Nation joins the WA
- `apply`: Nation applies to join the WA
- `cte`: Nation ceases to exist
- `delegate`: Regional delegate changes
- `feature`: Region (or its map) is featured
- `found`: Nation founded or refounded
- `join`: Nation moves into region
- `kick`: Nation kicked from WA
- `leave`: Nation leaves region
- `resign`: Nation resigns from the WA
- `rmb`: New RMB post
- `wa-discard`: WA resolution is discarded at vote
- `wa-fail`: WA resolution is defeated
- `wa-floor`: WA resolution enters the voting floor
- `wa-pass`: WA resolution passes
- `wa-submit`: WA proposal is submitted

**unless_event (string[])**

If the event name is contained in this array, the event will not match the rule. If any element of this array starts with a "#" character, the rest of the string will indicate a regex pattern instead.

If an event name is in both `if_event` and `unless_event`, `unless_event` will prevail, i.e. the event will not match.

For example, `unless_event = ["#wa-.+"]` will exclude any events that match the regex pattern `wa-.+` (for example "wa-floor").

**if_nation (string[])**

Only checked if the event has a nation.

If the nation name is contained in this array, the event will match the rule (provided any other conditions match). Otherwise, it will not. If any element of this array starts with a "#" character, the rest of the string will indicate a regex pattern instead. Nation names are always checked in lowercase, with spaces replaced by underscores.

For example, the below conditions will match any WA admission from either "merethin" or a nation whose name starts with "starlight":
```
if_event = ["admit"]
if_nation = ["merethin", "#starlight_.+"]
```

**unless_nation (string[])**

Only checked if the event has a nation.

If the nation name is contained in this array, the event will not match the rule. If any element of this array starts with a "#" character, the rest of the string will indicate a regex pattern instead. Nation names are always checked in lowercase, with spaces replaced by underscores.

If a nation name is in both `if_nation` and `unless_nation`, `unless_nation` will prevail, i.e. the event will not match.

For example, `unless_nation = ["#^.+_[0-9]+$"]` will exclude any nation whose name ends with a space followed by a number.

**if_region (string[])**

Only checked if the event has a region.

If the region name is contained in this array, the event will match the rule (provided any other conditions match). Otherwise, it will not. If any element of this array starts with a "#" character, the rest of the string will indicate a regex pattern instead. Region names are always checked in lowercase, with spaces replaced by underscores.

For example, the below conditions will match any nation moving to either Starlight or Horizon:
```
if_event = ["join"]
if_region = ["starlight", "horizon"]
```

**unless_region (string[])**

Only checked if the event has a region.

If the region name is contained in this array, the event will not match the rule. If any element of this array starts with a "#" character, the rest of the string will indicate a regex pattern instead. Region names are always checked in lowercase, with spaces replaced by underscores.

If a nation name is in both `if_region` and `unless_region`, `unless_region` will prevail, i.e. the event will not match.

For example, `unless_region = ["#^.+_[0-9]+$"]` will exclude any region whose name ends with a space followed by a number.

**if_content (string[])**

Only checked if the event has content (currently, only RMB posts).

If the content is contained in this array, the event will match the rule (provided any other conditions match). Otherwise, it will not. If any element of this array starts with a "#" character, the rest of the string will indicate a regex pattern instead.

Since content can be any string, the main usecase for this condition is to check whether it matches certain regex patterns, as opposed to verbatim strings.

For example, `if_content = ["#Roleplay Post"]` will match any event whose content contains the string "Roleplay Post".

**unless_content (string[])**

Only checked if the event has content (currently, only RMB posts).

If the content is contained in this array, the event will not match the rule. If any element of this array starts with a "#" character, the rest of the string will indicate a regex pattern instead.

Since content can be any string, the main usecase for this condition is to check whether it matches certain regex patterns, as opposed to verbatim strings.

For example, `unless_content = ["#OOC"]` will exclude any event whose content contains the string "OOC".

**if_wa (bool)**

Only checked if the event has a nation.

If this condition is present and equal to false, the event will only match the rule when the nation is not a WA member. If equal to true, the event will only match the rule when the nation _is_ a WA member.

### Parameters

**webhook (string, required)**

Name of the webhook to send an event to, if it matches the rule. Must be one of the names specified in the `webhooks` section above.

**color (string)**

Hex code to use as the embed's sidebar color when displaying an event. If not present, defaults to gray.

**role_mentions (string[])**

A list of roles to ping when an event matches the rule. Must be names specified in the `roles` section above.

**user_mentions (string[])**

A list of users to ping when an event matches the rule. Must be names specified in the `users` section above.

**links (string[])**

A list of link buttons to add to the end of the message. Current valid link names are:

- `endorse`: Generates a link to endorse a nation.
- `vote`: Generates a link to open the vote page (only shows up on `wa-floor` events).
- `post`: Generates a link to view a RMB post (only shows up on `rmb` events).
- `quote`: Generates a link to quote a RMB post (only shows up on `rmb` events).

**content_limit (number)**

For events with content (at the moment, only RMB posts), controls how much of that content to display.

By default, RMB events display up to 4096 characters (including formatting), as that is the limit imposed by Discord on embed descriptions. This parameter can be used to display a smaller amount of content (but not more).

## Tips

**If an event matches several rules, it does not pick the last one! All matching rules will be processed and a message will be sent to each of their webhooks.**

If you would like to override a certain event depending on a condition, create a separate rule and add a condition to exclude it to the original rule.

For example, if you would like to display all moves to a region, but ping a role and add a button if the nation who moved is a WA member:
```
[rule.join_nonwa]
if_event = ["join"]
if_region = ["examplelandia"]
if_wa = false
webhook = "main"

[rule.join_wa]
if_event = ["join"]
if_region = ["examplelandia"]
if_wa = true
webhook = "main"
role_mentions = ["role"]
links = ["endorse"]
```

Here, the `if_wa` condition controls which rule is selected.

If you would link to redirect certain RMB posts to another webhook, but still display all others:

```
[rule.normal_rmb]
if_event = ["rmb"]
if_region = ["examplelandia"]
unless_content = ["#RP Post"]
webhook = "main"

[rule.rp_rmb]
if_event = ["rmb"]
if_region = ["examplelandia"]
if_content = ["#RP Post"]
webhook = "rp"
```

Here, the mirrored `unless_content` and `if_content` conditions control which rule is selected.