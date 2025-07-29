with

international_trends as (
    select 
        week,
        refresh_date,
        'international_rising' as trend_type,
        count(*) as total_terms,
        avg(score) as avg_score,
        avg(percent_gain) as avg_percent_gain,
        max(percent_gain) as max_percent_gain,
        count(distinct country_name) as unique_countries
    from {{ ref('stg_international_top_rising_terms') }}
    group by week, refresh_date
    
    union all
    
    select 
        week,
        refresh_date,
        'international_top' as trend_type,
        count(*) as total_terms,
        avg(score) as avg_score,
        null as avg_percent_gain,
        null as max_percent_gain,
        count(distinct country_name) as unique_countries
    from {{ ref('stg_international_top_terms') }}
    group by week, refresh_date
),

us_trends as (
    select 
        week,
        refresh_date,
        'us_rising' as trend_type,
        count(*) as total_terms,
        avg(score) as avg_score,
        avg(percent_gain) as avg_percent_gain,
        max(percent_gain) as max_percent_gain,
        count(distinct dma_name) as unique_geos
    from {{ ref('stg_top_rising_terms') }}
    group by week, refresh_date
    
    union all
    
    select 
        week,
        refresh_date,
        'us_top' as trend_type,
        count(*) as total_terms,
        avg(score) as avg_score,
        null as avg_percent_gain,
        null as max_percent_gain,
        count(distinct dma_name) as unique_geos
    from {{ ref('stg_top_terms') }}
    group by week, refresh_date
),

combined_trends as (
    select 
        week,
        refresh_date,
        trend_type,
        total_terms,
        avg_score,
        avg_percent_gain,
        max_percent_gain,
        unique_countries as unique_geos
    from international_trends
    
    union all
    
    select 
        week,
        refresh_date,
        trend_type,
        total_terms,
        avg_score,
        avg_percent_gain,
        max_percent_gain,
        unique_geos
    from us_trends
),

final as (
    select
        week,
        refresh_date,
        trend_type,
        total_terms,
        round(avg_score, 2) as avg_score,
        round(avg_percent_gain, 2) as avg_percent_gain,
        max_percent_gain,
        unique_geos,
        
        -- Add trend categorization
        case 
            when trend_type like '%rising%' then 'Rising Trends'
            else 'Top Trends'
        end as trend_category,
        
        case 
            when trend_type like 'international%' then 'International'
            else 'US Market'
        end as market_scope
        
    from combined_trends
)

select * from final
order by week desc, trend_type