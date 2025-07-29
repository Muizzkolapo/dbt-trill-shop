{% docs __overview__ %}
# Google Trends Analytics Project

This dbt project models Google Trends data from the `project-kphash.google_trends` BigQuery dataset, providing comprehensive analysis of search trends across international and US markets.

## Project Structure

- **Staging Models**: Clean and standardize raw Google Trends data
- **Mart Models**: Business-ready models for analysis and reporting

## Data Sources

All data comes from the `project-kphash.google_trends` BigQuery dataset, which contains:
- International rising and top search terms by country/region
- US market rising and top search terms by DMA (Designated Market Area)

## Key Models

- `trending_terms_analysis`: Unified analysis of rising search terms with growth categorization
- `top_terms_comparison`: Comparison of top terms across markets with interest levels
- `weekly_trends_summary`: Weekly aggregated metrics across all trend types

{% enddocs %}

{% docs source_google_trends %}
Google Trends data from the `project-kphash.google_trends` BigQuery dataset. This dataset provides comprehensive search trend information across international markets and US DMAs, updated on a regular basis.

The data includes both rising terms (showing rapid growth) and top terms (showing highest overall search volume) with associated metrics like search scores, rankings, and percentage gains.
{% enddocs %}

{% docs table_international_top_rising_terms %}
## International Rising Search Terms

This table contains search terms that are experiencing rapid growth in popularity across different countries and regions worldwide. 

**Key Features:**
- **Geographic Coverage**: Global countries and regions
- **Trend Type**: Rising/emerging search terms
- **Time Partitioning**: Partitioned by `refresh_date` for optimal query performance
- **Growth Metrics**: Includes percentage gain to measure trend velocity

**Business Use Cases:**
- Identify emerging global trends and viral content
- Monitor regional variations in search behavior
- Track cultural and social movements across countries
- Inform global marketing and content strategies

**Data Freshness**: Updated regularly based on Google Trends data availability
{% enddocs %}

{% docs table_international_top_terms %}
## International Top Search Terms

This table contains the most popular search terms by overall search volume across different countries and regions worldwide.

**Key Features:**
- **Geographic Coverage**: Global countries and regions  
- **Trend Type**: Highest volume search terms
- **Time Partitioning**: Partitioned by `refresh_date` for optimal query performance
- **Volume Metrics**: Includes search scores and rankings

**Business Use Cases:**
- Understand mainstream search behavior by geography
- Benchmark content performance against popular terms
- Identify market opportunities in different regions
- Support international SEO and marketing strategies

**Data Freshness**: Updated regularly based on Google Trends data availability
{% enddocs %}

{% docs table_top_rising_terms %}
## US Rising Search Terms by DMA

This table contains search terms experiencing rapid growth across US Designated Market Areas (DMAs).

**Key Features:**
- **Geographic Coverage**: US DMAs (media markets)
- **Trend Type**: Rising/emerging search terms
- **Time Partitioning**: Partitioned by `refresh_date` for optimal query performance
- **Growth Metrics**: Includes percentage gain to measure trend velocity

**Business Use Cases:**
- Identify emerging US market trends
- Target marketing campaigns by media market
- Track regional variations in US consumer behavior
- Support local advertising and content strategies

**Data Freshness**: Updated regularly based on Google Trends data availability
{% enddocs %}

{% docs table_top_terms %}
## US Top Search Terms by DMA

This table contains the most popular search terms by overall search volume across US Designated Market Areas (DMAs).

**Key Features:**
- **Geographic Coverage**: US DMAs (media markets)
- **Trend Type**: Highest volume search terms  
- **Time Partitioning**: Partitioned by `refresh_date` for optimal query performance
- **Volume Metrics**: Includes search scores and rankings

**Business Use Cases:**
- Understand mainstream US search behavior by market
- Benchmark content against popular terms by region
- Support local SEO and marketing efforts
- Analyze market penetration across US regions

**Data Freshness**: Updated regularly based on Google Trends data availability
{% enddocs %}

{% docs model_trending_terms_analysis %}
## Trending Terms Analysis

A unified mart model that combines rising search terms from both international and US markets, providing comprehensive trend analysis with categorized growth metrics.

**Transformation Logic:**
1. **Data Unification**: Combines international and US rising terms data
2. **Standardization**: Normalizes geographic fields across data sources
3. **Categorization**: Adds rank tiers and growth categories for analysis
4. **Business Logic**: Applies consistent business rules across markets

**Key Features:**
- **Unified View**: Single table for all rising trends analysis
- **Growth Categories**: Terms classified by growth rate (Explosive, Very High, High, Moderate, Low)
- **Rank Tiers**: Terms grouped by ranking performance (Top 5, Top 10, Top 25, Other)
- **Market Scope**: Clear distinction between international and US market data

**Use Cases:**
- Cross-market trend comparison and analysis
- Identify high-growth opportunities across regions
- Support global vs local marketing strategy decisions
- Track viral content and emerging topics worldwide
{% enddocs %}

{% docs model_top_terms_comparison %}
## Top Terms Comparison

A mart model that provides standardized comparison of top search terms across international and US markets, with categorized interest levels.

**Transformation Logic:**
1. **Data Unification**: Combines international and US top terms data
2. **Standardization**: Normalizes geographic fields for comparison
3. **Categorization**: Adds rank categories and interest levels
4. **Business Logic**: Applies consistent analysis framework

**Key Features:**
- **Cross-Market View**: Compare top terms between international and US markets
- **Interest Levels**: Terms classified by search score intensity
- **Rank Categories**: Special highlighting of #1 terms and top performers
- **Geographic Flexibility**: Works with both country-level and DMA-level data

**Use Cases:**
- Benchmark performance against market leaders
- Identify content gaps and opportunities
- Support competitive analysis across markets
- Guide content strategy and SEO priorities
{% enddocs %}

{% docs model_weekly_trends_summary %}
## Weekly Trends Summary

An aggregated mart model that provides weekly metrics across all Google Trends data types, enabling trend analysis over time.

**Transformation Logic:**
1. **Data Aggregation**: Weekly rollups of all trend types
2. **Metric Calculation**: Average scores, growth rates, and geographic coverage
3. **Trend Categorization**: Classification by trend type and market scope
4. **Time Series Preparation**: Optimized for temporal analysis and reporting

**Key Features:**
- **Time Series Ready**: Weekly aggregated data perfect for dashboards
- **Comprehensive Coverage**: All trend types in a single view
- **Key Metrics**: Average scores, growth rates, geographic diversity
- **Trend Classification**: Clear categorization for filtering and analysis

**Use Cases:**
- Track overall trend health and activity over time
- Monitor seasonal patterns in search behavior
- Measure geographic diversity of trending topics
- Support executive reporting and trend dashboards
{% enddocs %}

{% docs col_refresh_date %}
The date when the Google Trends data was last refreshed/updated. This field is used for data partitioning and ensures you're working with the most current available data.
{% enddocs %}

{% docs col_country_code %}
ISO country code (e.g., 'US', 'GB', 'DE') representing the country where the search trend was observed. Used for geographic filtering and analysis.
{% enddocs %}

{% docs col_country_name %}
Full name of the country where the search trend was observed (e.g., 'United States', 'United Kingdom', 'Germany'). More readable than country codes for reporting.
{% enddocs %}

{% docs col_region_code %}
Code representing a specific region within a country. This provides more granular geographic analysis than country-level data alone.
{% enddocs %}

{% docs col_region_name %}
Full name of the region within a country. Provides readable geographic context for sub-country analysis.
{% enddocs %}

{% docs col_dma_id %}
Unique identifier for a US Designated Market Area (DMA). DMAs represent television market areas and are commonly used for media planning and advertising.
{% enddocs %}

{% docs col_dma_name %}
Name of the US Designated Market Area (e.g., 'New York', 'Los Angeles', 'Chicago'). DMAs represent television market areas used for local advertising and media planning.
{% enddocs %}

{% docs col_term %}
The actual search term or query that users entered into Google. This is the core data point for understanding what people are searching for.
{% enddocs %}

{% docs col_week %}
The specific week (date) that the search trend data represents. Google Trends data is typically aggregated by week for trend analysis.
{% enddocs %}

{% docs col_score %}
Google's search interest score, typically ranging from 0-100, where 100 represents peak search interest for the term during the time period. Higher scores indicate more search volume.
{% enddocs %}

{% docs col_rank %}
The ranking position of the search term within its category and geographic area. Rank 1 represents the most popular/trending term for that week and location.
{% enddocs %}

{% docs col_percent_gain %}
The percentage increase in search volume for rising terms compared to the previous period. This metric helps identify viral content and rapidly emerging trends. Only available for "rising" trend data.
{% enddocs %}

{% docs col_scope %}
Indicates the market scope of the data - either 'international' for global country/region data or 'us_dma' for US market area data. Used to distinguish between data sources in unified models.
{% enddocs %}

{% docs col_geo_name %}
Standardized geographic area name that works across both international (country names) and US (DMA names) data. Enables consistent geographic analysis in unified models.
{% enddocs %}

{% docs col_geo_code %}
Standardized geographic area code that works across both international (country codes) and US (DMA IDs) data. Provides consistent geographic identifiers in unified models.
{% enddocs %}

{% docs col_rank_tier %}
Categorized ranking performance: 'Top 5' (ranks 1-5), 'Top 10' (ranks 6-10), 'Top 25' (ranks 11-25), or 'Other' (ranks 26+). Simplifies analysis of high-performing terms.
{% enddocs %}

{% docs col_growth_category %}
Categorized growth rate based on percent_gain: 'Explosive (1000%+)', 'Very High (500-999%)', 'High (200-499%)', 'Moderate (100-199%)', or 'Low (<100%)'. Helps identify different types of trending content.
{% enddocs %}

{% docs col_rank_category %}
Categorized ranking with special emphasis on top performers: '#1 Term' (rank 1), 'Top 5' (ranks 2-5), 'Top 10' (ranks 6-10), or 'Other' (ranks 11+). Highlights market-leading content.
{% enddocs %}

{% docs col_interest_level %}
Categorized search interest based on score: 'Very High Interest' (80+), 'High Interest' (60-79), 'Moderate Interest' (40-59), 'Low Interest' (20-39), 'Very Low Interest' (<20).
{% enddocs %}

{% docs col_trend_type %}
Specific type of trend data: 'international_rising', 'international_top', 'us_rising', or 'us_top'. Distinguishes between different Google Trends datasets in aggregated models.
{% enddocs %}

{% docs col_trend_category %}
High-level trend classification: 'Rising Trends' for emerging/growing terms or 'Top Trends' for highest volume terms. Simplifies filtering and analysis.
{% enddocs %}

{% docs col_market_scope %}
Market classification: 'International' for global data or 'US Market' for US-specific data. Enables market-level filtering and comparison.
{% enddocs %}