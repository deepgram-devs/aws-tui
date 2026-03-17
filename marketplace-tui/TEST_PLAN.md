# marketplace-tui Manual Test Plan

## Prerequisites

- AWS credentials configured with Marketplace Catalog permissions
- Terminal of at least 120×40 characters recommended
- Build: `cargo build -p marketplace-tui`

---

## 1. Startup & Product List

| # | Test | Expected |
|---|------|----------|
| 1.1 | Launch with valid AWS credentials | Products list loads; status bar shows count |
| 1.2 | Launch with invalid/missing credentials | Error displayed in status bar; app still opens |
| 1.3 | Press `?` | Help popup appears over product list |
| 1.4 | Press any key in help popup | Popup closes |
| 1.5 | Type characters | Filter applied; product count updates |
| 1.6 | Type to filter, press `Esc` | Filter cleared; all products restored |
| 1.7 | Press `↑` / `↓` | Selection moves; highlight follows |
| 1.8 | Press `j` / `k` | Selection moves (vim keys) |
| 1.9 | Press `PgUp` / `PgDn` | Selection jumps by 10 rows |
| 1.10 | Navigate past end of list | Wraps to beginning |
| 1.11 | Press `R` | Products reload; spinner visible briefly |
| 1.12 | Press `l` | Log overlay opens |
| 1.13 | Press `Esc` or `l` in log | Log closes |
| 1.14 | Press `y` with product selected | Entity ID copied to clipboard; status confirms |
| 1.15 | Press `:` or `Space` | Command palette opens |
| 1.16 | Mouse scroll up/down | Product selection moves |

---

## 2. Command Palette

| # | Test | Expected |
|---|------|----------|
| 2.1 | Open palette (`:`) and type "create" | Filtered to matching commands |
| 2.2 | Clear input | All commands shown |
| 2.3 | Navigate with `↑` / `↓` | Selection moves in palette |
| 2.4 | Press `Enter` on "Refresh" | Palette closes; products refresh |
| 2.5 | Press `Esc` | Palette closes; returns to previous view |
| 2.6 | Each command has shortcut shown | Shortcut column populated |

---

## 3. Product Detail

| # | Test | Expected |
|---|------|----------|
| 3.1 | Press `Enter` on product | Detail view opens; spinner shows while loading |
| 3.2 | Detail loads | Overview tab shows all product fields |
| 3.3 | Press `Tab` | Cycles through Overview → Versions → Pricing → Allowlist |
| 3.4 | Versions tab | Lists delivery options with visibility color coding |
| 3.5 | Pricing tab | Shows pricing terms |
| 3.6 | Allowlist tab | Shows allowed account IDs |
| 3.7 | Press `Esc` | Returns to product list |
| 3.8 | Press `y` | Copies entity ID to clipboard |
| 3.9 | Press `R` | Refreshes product detail |

---

## 4. Create Product Wizard

| # | Test | Expected |
|---|------|----------|
| 4.1 | Press `c` from product list | Wizard opens at Step 1/4 |
| 4.2 | Press `Enter` with empty title | Validation error shown |
| 4.3 | Fill title >255 chars | Validation error shown |
| 4.4 | Fill valid title + short desc, press `Enter` | Advances to Step 2/4 |
| 4.5 | Press `Tab` | Switches between fields in current step |
| 4.6 | Press `Shift+Tab` | Goes back to previous step |
| 4.7 | Complete all steps to Review | Summary shows all entered fields |
| 4.8 | Review shows EULA and $0.01 pricing note | Correct defaults displayed |
| 4.9 | Press `Enter` on Review | Changeset submitted; status message shown |
| 4.10 | After submit | Returns to product list; changeset visible in `s` view |
| 4.11 | Press `Esc` at any step | Wizard cancelled; returns to product list |

---

## 5. Create Version Wizard

| # | Test | Expected |
|---|------|----------|
| 5.1 | Open product detail, press `v` | Version wizard opens |
| 5.2 | Press `Enter` with empty title | Validation error |
| 5.3 | Fill title + release notes, advance | Goes to Review step |
| 5.4 | Fill optional Model Package ARN | Shown in review |
| 5.5 | Leave ARN empty | Still works; ARN omitted from submission |
| 5.6 | Submit | Changeset submitted; returns to product detail |

---

## 6. Metadata Editor

| # | Test | Expected |
|---|------|----------|
| 6.1 | Open product detail, press `e` | Metadata editor opens with current values pre-populated |
| 6.2 | Navigate with `↑` / `↓` | Field selection changes |
| 6.3 | Press `Enter` on a field | Input buffer shows current value; cursor appears |
| 6.4 | Edit value, press `Enter` | Value updated in list |
| 6.5 | Press `Esc` while editing | Edit cancelled; value unchanged |
| 6.6 | Press `Ctrl+S` | Changeset submitted; returns to product detail |
| 6.7 | Press `Esc` from editor | Returns to product detail without saving |

---

## 7. Allowlist Manager

| # | Test | Expected |
|---|------|----------|
| 7.1 | Press `w` from product detail | Allowlist manager opens |
| 7.2 | Press `a` | Input field activates |
| 7.3 | Type non-numeric chars | Only digits accepted |
| 7.4 | Type < or > 12 digits | Red input color; error hint |
| 7.5 | Type exactly 12 digits | Green input color; valid indicator |
| 7.6 | Press `Enter` with valid ID | Account added to list; changeset submitted |
| 7.7 | Try to add duplicate | Not added again |
| 7.8 | Press `d` on selected account | Account removed; changeset submitted |
| 7.9 | Press `Esc` in input | Input cleared; returns to list navigation |
| 7.10 | Press `Esc` from manager | Returns to product detail |

---

## 8. Pricing Editor

| # | Test | Expected |
|---|------|----------|
| 8.1 | Press `p` from product detail | Pricing editor opens with current price |
| 8.2 | Press `Tab` | Cycles through Price → Currency → Dimension |
| 8.3 | Type invalid price (non-numeric) | Red color on price field |
| 8.4 | Type valid price (e.g. 0.01) | White color on price field |
| 8.5 | Press `Enter` with invalid price | Error message shown |
| 8.6 | Press `Enter` with valid price | Changeset submitted; returns to product detail |
| 8.7 | Currency field limits to 3 uppercase chars | Input constrained |
| 8.8 | Press `Esc` | Returns without saving |

---

## 9. Version List & Restrict

| # | Test | Expected |
|---|------|----------|
| 9.1 | Press `V` from product detail | Version list opens |
| 9.2 | Navigate with `↑` / `↓` | Selection moves |
| 9.3 | Each version shows ID, visibility, release notes | Information visible |
| 9.4 | Press `r` on a version | Confirmation dialog appears |
| 9.5 | Press `n` or `Esc` in confirm | Cancelled; returns to version list |
| 9.6 | Press `y` in confirm | Restrict changeset submitted |
| 9.7 | Press `y` on version in version list | Version ID copied to clipboard |
| 9.8 | Press `Esc` | Returns to product detail |

---

## 10. Change Set Status View

| # | Test | Expected |
|---|------|----------|
| 10.1 | Press `s` from any view | Change set status overlay opens |
| 10.2 | Submit any operation | New entry appears with "Preparing" status |
| 10.3 | Wait 5 seconds | Status updates automatically |
| 10.4 | Successful changeset | Status turns green "Succeeded" |
| 10.5 | Failed changeset | Status turns red; failure description shown |
| 10.6 | Press `R` | Forces immediate poll of all pending changesets |
| 10.7 | Header spinner | Visible while any changeset is non-terminal |
| 10.8 | Press `Esc` | Returns to previous view |

---

## 11. Terminal Reset

| # | Test | Expected |
|---|------|----------|
| 11.1 | Press `q` | App exits cleanly; terminal prompt visible |
| 11.2 | Press `Ctrl+C` | App exits cleanly; terminal prompt visible |
| 11.3 | After exit | No leftover alternate screen or raw mode artifacts |

---

## 12. Mouse Controls

| # | Test | Expected |
|---|------|----------|
| 12.1 | Scroll up/down in product list | Selection moves |
| 12.2 | Scroll up/down in version list | Selection moves |
| 12.3 | Scroll up/down in log viewer | Log scrolls |
