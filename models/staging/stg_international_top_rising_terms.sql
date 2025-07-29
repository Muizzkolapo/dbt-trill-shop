with

source as (

    select * from {{ source('ecom', 'international_top_rising_terms') }}

),

renamed as (

    select
        ----------  ids
        country_code,
        region_code,
        
        ---------- text
        country_name,
        region_name,
        term,
        
        ---------- dates
        refresh_date,
        week,
        
        ---------- numerics
        score,
        rank,
        percent_gain

    from source

)

select * from renamed