with

international_rising as (
    select 
        'international' as scope,
        country_name as geo_name,
        country_code as geo_code,
        region_name,
        term,
        week,
        refresh_date,
        score,
        rank,
        percent_gain
    from {{ ref('stg_international_top_rising_terms') }}
),

us_rising as (
    select 
        'us_dma' as scope,
        dma_name as geo_name,
        cast(dma_id as string) as geo_code,
        null as region_name,
        term,
        week,
        refresh_date,
        score,
        rank,
        percent_gain
    from {{ ref('stg_top_rising_terms') }}
),

all_rising_terms as (
    select * from international_rising
    union all
    select * from us_rising
),

final as (
    select
        scope,
        geo_name,
        geo_code,
        region_name,
        term,
        week,
        refresh_date,
        score,
        rank,
        percent_gain,
        
        -- Add calculated fields
        case 
            when rank <= 5 then 'Top 5'
            when rank <= 10 then 'Top 10'
            when rank <= 25 then 'Top 25'
            else 'Other'
        end as rank_tier,
        
        case 
            when percent_gain >= 1000 then 'Explosive (1000%+)'
            when percent_gain >= 500 then 'Very High (500-999%)'
            when percent_gain >= 200 then 'High (200-499%)'
            when percent_gain >= 100 then 'Moderate (100-199%)'
            else 'Low (<100%)'
        end as growth_category
        
    from all_rising_terms
)

select * from final