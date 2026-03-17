# marketplace-tui

A terminal user interface for managing AWS Marketplace machine learning product listings for SageMaker.

## Product Functions

- Create new AWS Marketplace machine learning products (4-step wizard)
- Add new product versions (4-step wizard)
- List the existing product versions and visibility
- Restrict existing product versions for deprecation
- Whitelist (aka. allow list) products for zero or more 12-digit AWS account IDs
  - Add allow list items
  - Remove allow list items
- Update metadata fields for products and product versions
- View and update pricing details for an existing AWS Marketplace product
- AWS Marketplace Management change set requests
  - View a list of all the pending change sets and their status
  - Change sets are polled for status every 5 seconds in the background

## Requirements

- Ensure all required fields are filled out properly, depending on the product or product version being created.
- Ensure that there is client side validation of field inputs before sending the input parameters to the AWS APIs
- Ensure product tag key-value pairs are added as appropriate, such as:
  - Date product originally created
  - Who created the product (username)
  - Which product line it's for (Deepgram: STT Speech-to-Text, TTS Text-to-Speech)
- Use `--debug-file <path>` CLI flag to write raw `DescribeEntity` JSON responses to a file for debugging

## AWS Marketplace API Notes

The following field structures have been validated against the live API:

### DescribeEntity (ML Product) — JSON structure

```
{
  "Description": { "ProductTitle", "ProductCode", "ShortDescription", "LongDescription",
                   "Visibility", "ProductState", "Categories", "SearchKeywords" },
  "PromotionalResources": { "LogoUrl", "Videos" },
  "SupportInformation": { "Description", "Resources" },
  "Targeting": { "PositiveTargeting": { "BuyerAccounts": [...] } },
  "Dimensions": [ { "Name", "Description", "Key", "Unit", "Types" } ],
  "Versions": [ { "Id", "VersionTitle", "ReleaseNotes", "CreationDate",
                  "Sources": [ { "ModelPackageArn" } ],
                  "DeliveryOptions": [ { "Id", "Visibility", ... } ] } ],
  "RegionAvailability": { ... }
}
```

- Pricing: returned under `Dimensions` (metered billing), not a `Terms` array
- Refund policy: not returned by the API
- Allowlist accounts: `Targeting.PositiveTargeting.BuyerAccounts` (field name is `BuyerAccounts`, not `BuyerAccountIds`)

### StartChangeSet — UpdateTargeting

```json
{ "PositiveTargeting": { "BuyerAccounts": ["123456789012"] } }
```

### StartChangeSet — AddDeliveryOptions

```json
{
  "Version": { "VersionTitle": "...", "ReleaseNotes": "..." },
  "DeliveryOptions": [{
    "Details": {
      "SageMakerModelPackageDeliveryOptionDetails": {
        "SageMakerModelPackageArn": "arn:aws:sagemaker:...",
        "AccessRoleArn": "arn:aws:iam::...:role/...",
        "UsageInstructions": "...",
        "RecommendedInstanceTypes": {
          "RealtimeInference": "ml.g5.2xlarge",
          "BatchTransform": "ml.m5.xlarge"
        },
        "RepositoryUrl": "https://...",
        "SampleNotebookUrl": "https://...",
        "InputProperties": {
          "Description": "...",
          "SampleInput": { "RealtimeInferenceUrl": "...", "BatchTransformUrl": "..." }
        },
        "OutputProperties": {
          "Description": "...",
          "SampleOutput": { "RealtimeInferenceText": "...", "BatchTransformText": "..." }
        }
      }
    }
  }]
}
```

### Product Details defaults for new listings

- Initial pricing: $0.01 USD
- Refund policy: N/A
- EULA: Standard AWS Marketplace Contract (not custom)

## AWS Credentials

Uses the default AWS credential chain (env vars, `~/.aws/credentials`, IAM roles). The Marketplace Catalog API endpoint is global/`us-east-1`.

## Resources

- Create machine learning products: https://docs.aws.amazon.com/marketplace/latest/userguide/machine-learning-products.html
- API reference for machine learning products: https://docs.aws.amazon.com/marketplace/latest/APIReference/work-with-ml-products.html
