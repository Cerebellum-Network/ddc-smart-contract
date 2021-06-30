# Specification of the DDC Payment System

## Actors

DDC, for Decentralized Data Cloud, is an infrastructure service.

Apps are the users of DDC. They are developers of applications built on top of the DDC infrastructure.

SC is the Smart Contract which manages the payment system for this service.

DDNs are the nodes (servers) of the service. DDNs read regularly from the SC.

Inspectors are oracles that inspect DDNs and reports their activities to the SC.

Operator is a privileged actor who configures the SC ("owner" in SC code).

## Flow of payments

An App subscribes into the service and pays tokens to the SC. There is a web UI showing prices and buttons to subscribe.

The App uses the DDC service by connecting to DDNs. Requests are authenticated by signatures from the same key pair as
for the subscription.

Each DDN confirms the status of an App subscription regularly before accepting service.

A list of DDNs can be discovered from the SC. The Operator configures this list. DDNs authorize each other for internal
operations based on this list.

The price of services is coded into "tiers". A tier represents an amount of reserved DDC capacity. Reserved capacity is
to be paid regardless of actual usage. Prices are given for a period of 31 days. The Operator configures the tiers.

An App can prepay for future service. The prepayments are automatically consumed over time (auto-renew). The tier of
service can be changed. An App can be refunded its unused balance by cancelling its subscription at any time.

The Operator can withdraw all revenues that were earned from Apps so far, but not prepayments. It is possible to
attribute revenues to individual DDNs for offchain accounting.

Malicious Operator, DDNs, and Inspectors (all at once) may cause the service to be provided incorrectly or not at all,
but they cannot create more payment charges to Apps than expected at subscription time.

## Flow of metrics

Inspectors monitor DDNs regularly (e.g., 10 minutes). They collects so-called metrics, that is the usage of resources
such as bandwidth and storage. Each DDN reports metrics about the service provided to each App.

Metrics are aggregated per App over all DDNs. Presently this is not connected to the subscriptions, but it should be in
the future, to implement a pay-as-go model.

Separately, metrics are aggregated per DDN over all Apps. This can be used to attribute revenues to DDNs.

Metrics are measured at a granularity of 1 day, and retained for 31 days.

The Inspectors also report the fact that a DDN becomes offline or back online. It is possible to watch the SC for the
current status of DDNs, and to calculate historical availability (% downtime).

Metrics from multiple Inspectors for a given day are merged by taking the median values.

## Assumptions

**The system requires the following trust assumptions. These are considered limitations to remove or mitigate in the
future.**

The Operator is essentially the sole provider of DDC running all DDNs, or he is in a trusted relationship with all other
providers.

All DDNs are running the official software. DDNs can have non-malicious failures.

A majority of Inspectors that are regularly active are running the official software.

Inspectors are supposed to be selected by some governance mechanism. E.g., by staking and nomination on the Cere chain,
or by mutual agreement of Apps and DDNs. Currently, this is emulated by admin functions of the Operator.
