with

international_top as (
    select 
        'international' as scope,
        country_name as geo_name,
        country_code as geo_code,
        region_name,
        term,
        week,
        refresh_date,
        score,
        rank
    from {{ ref('stg_international_top_terms') }}
),

us_top as (
    select 
        'us_dma' as scope,
        dma_name as geo_name,
        cast(dma_id as string) as geo_code,
        null as region_name,
        term,
        week,
        refresh_date,
        score,
        rank
    from {{ ref('stg_top_terms') }}
),

all_top_terms as (
    select * from international_top
    union all
    select * from us_top
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
        
        -- Add calculated fields
        case 
            when rank = 1 then '#1 Term'
            when rank <= 5 then 'Top 5'
            when rank <= 10 then 'Top 10'
            else 'Other'
        end as rank_category,
        
        case 
            when score >= 80 then 'Very High Interest'
            when score >= 60 then 'High Interest'
            when score >= 40 then 'Moderate Interest'
            when score >= 20 then 'Low Interest'
            else 'Very Low Interest'
        end as interest_level
        
    from all_top_terms
)

select * from final