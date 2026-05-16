{% if !has_results %}No results found.{% else -%}
{% for section in general_sections %}## {{ section.title }}

{% for item in section.items %}{{ item.index }}. **[{{ item.title }}]({{ item.url }})**
{% if let Some(s) = item.snippet %}   - Snippet: {{ s }}
{% endif %}{% if let Some(t) = item.time %}   - Published: {{ t }}
{% endif %}{% if let Some(d) = item.duration %}   - Duration: {{ d }}
{% endif %}{% if let Some(l) = item.language %}   - Language: {{ l }}
{% endif %}{% if item.paywalled %}   - Paywalled: true
{% endif %}{% if let Some(a) = item.ai_content %}   - AI Content: {{ a }}
{% endif %}{% endfor %}
{% endfor %}{% if !image_results.is_empty() %}## Images

{% for item in image_results %}{{ item.index }}. **[{{ item.title }}]({{ item.url }})**
   - Image: {{ item.image_url }} ({{ item.width }}x{{ item.height }})
{% endfor %}
{% endif %}{% if !related_questions.is_empty() %}## Related Questions

{% for item in related_questions %}{{ item.index }}. **{{ item.question }}**
{% if let Some(s) = item.snippet %}    - [Answer]({{ item.url }}): {{ s }}
{% endif %}{% endfor %}
{% endif %}{% if !direct_answers.is_empty() %}## Direct Answer

{% for item in direct_answers %}{{ item.snippet }}

{% endfor %}{% endif %}{% if !infoboxes.is_empty() %}## Infobox

{% for item in infoboxes %}**[{{ item.title }}]({{ item.url }})**

{% if let Some(s) = item.snippet %}{{ s }}

{% endif %}{% for (key, value) in item.properties %}{{ key }}: {{ value }}
{% endfor %}
{% endfor %}{% endif %}{% if !related_searches.is_empty() %}## Related Searches

{% for item in related_searches %}- {{ item.title }}
{% endfor %}
{% endif %}{% if !weather.is_empty() %}## Weather

{% for item in weather %}{{ item.snippet }}
{% endfor %}
{% endif %}{% if !package_tracking.is_empty() %}## Package Tracking

{% for item in package_tracking %}- [Tracking Link]({{ item.url }})
{% endfor %}{% endif %}{% endif %}
