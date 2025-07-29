with

source as (

    select * from {{ source('ecom', 'top_rising_terms') }}

),

renamed as (

    select
        ----------  ids
        dma_id,
        
        ---------- text
        dma_name,
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