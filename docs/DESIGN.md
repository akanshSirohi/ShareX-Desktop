# ShareX Desktop design

Visual thesis: A precise, quiet desktop workspace with graphite surfaces, violet actions, and generous spacing.

Content plan: The file workspace is primary. A persistent sidebar organizes files, transfers, devices, plugins, and settings. A connection inspector explains server status and access. Root selection is an explicit configuration flow.

Interaction thesis: Staggered workspace entrance, a short inspector transition, and subtle file row hover states establish hierarchy. Reduced-motion settings disable all animation.

The application layout overrides the generated marketing recommendation. Use Segoe UI Variable/Segoe UI, semantic tables, visible keyboard focus, 44px primary controls, and one violet accent. Support light and dark themes. Never present invented files, devices, transfer speeds, or active sharing in the production interface.

The device connection dialog uses three labeled controls in one row: protocol, destination IP, and port. Prefill the local IPv4 network prefix when no previous destination exists. Keep the destination editable and never imply that this PC's own IP is the other device. QR codes and pasted connection URLs populate the same fields. Show validation within the dialog and open the selected device's portal in the default browser.

Access requests use the existing modal style, showing the browser name, IP address, requested plugin, and a session approval action. Browser approval comes before plugin approval. Requests remain on the Devices page when the popup is dismissed. Defer request popups while another dialog is open. Direct plugin links show a waiting page with password entry when needed, then open automatically after approval.

The sidebar uses the visible viewport height. Compact spacing below 800px and 700px keeps the privacy note, Settings, and profile visible at the minimum desktop height. Allow vertical sidebar scrolling for smaller windows or enlarged text instead of clipping controls.

Scrollbars follow the active theme across the page and nested panels. Use a slim rounded thumb, a transparent track, subtle hover contrast, and no arrow buttons. Keep standard thin scrollbars as the fallback for engines without WebKit scrollbar selectors.
