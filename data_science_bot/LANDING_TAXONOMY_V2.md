# Landing Taxonomy V2

Version: `nd_landing_taxonomy.v2`

This taxonomy replaces the previous generic landing-page grouping with a Nature's Diet-specific route map.

## Design Goals

1. Keep the dominant business surfaces out of `other`.
2. Separate exact offer landings from general product-detail pages.
3. Preserve a stable `landing_family` for science and experimentation.
4. Preserve a simpler `landing_page_group` for executive reporting.
5. Make control-versus-challenger landing experiments explicit and reproducible.

## Rule Order

Rules are evaluated top-to-bottom. Earlier matches win.

| Rule ID | Match Logic | Landing Family | Landing Page Group | Notes |
| --- | --- | --- | --- | --- |
| `home.root` | exact `/` | `home` | `home` | Default homepage baseline |
| `offer.bundle` | exact `/simply-raw-value-bundle-assortment` or `bundle|assortment` in route | `bundle_offer_lp` | `offer_landing` | Highest-value bundle offer entry |
| `offer.ready_raw` | `^/ready-raw` | `ready_raw_offer_lp` | `offer_landing` | Ready Raw offer family |
| `offer.simply_raw` | `^/simply-raw` excluding bundle rules | `simply_raw_offer_lp` | `offer_landing` | Current dominant paid landing control candidate |
| `product.detail` | `^/product-page/` | `product_detail_lp` | `product_detail` | PDP family |
| `catalog.category` | exact `/our-products`, `/dog-treats`, `/bone-broth` or `^/(collections?|shop)` | `category_or_catalog_lp` | `category_or_catalog` | Catalog browsing surfaces |
| `lead.freebook` | exact `/freebook-rawfeedingguide` or lead-magnet terms | `lead_magnet_lp` | `lead_magnet` | Non-transactional entry built for capture/nurture |
| `content.post` | `^/post/` or content/blog/learn routes | `content_lp` | `content` | Editorial education and SEO |
| `account.portal` | `^/account/` | `account_portal_lp` | `account_or_support` | Existing-customer/account surfaces |
| `brand.story` | exact `/our-story` or about/mission routes | `brand_story_lp` | `brand_or_info` | Brand trust pages |
| `support.policy` | contact/faq/policy/support routes | `support_or_policy_lp` | `account_or_support` | Support and policy pages |
| `cart.checkout` | `^/(cart|checkout)` | `cart_or_checkout_lp` | `cart_or_checkout` | Bottom-funnel operational pages |
| `unknown.missing` | no recoverable first page path | `unknown` | `unknown` | True measurement gap |
| `fallback.other` | any other matched route | `other_marketing_lp` | `other_marketing` | Temporary bucket to be driven down over time |

## Operational Interpretation

- `landing_family` is the scientist-facing classification used for experiments and cohort analysis.
- `landing_page_group` is the operator-facing rollup used in executive reports.
- `simply_raw_offer_lp` is a valid control candidate because it is the dominant existing paid landing family.
- `bundle_offer_lp` and `product_detail_lp` are valid challenger candidates for experiment design.
- A path in `other_marketing_lp` is not automatically bad data, but it should be promoted into a named family once it is decision-relevant.

## Tightening Targets

1. `other_marketing_lp` should be reduced below `15%` of sessions.
2. Purchase-bearing sessions in `other_marketing_lp` should trend toward zero.
3. Top paid landing paths should all map to a named `landing_family`.
4. `unknown` should reflect true missingness, not unmapped known routes.
